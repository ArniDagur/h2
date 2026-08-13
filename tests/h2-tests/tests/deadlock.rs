//! Stress test related to hyperium/h2#853 / PR #852.
//!
//! Concurrent POSTs with bodies while server max_concurrent_streams is lower
//! than client concurrency. Pre-#860 theory: capacity assigned to pending_open
//! streams could starve open streams (logical deadlock).
//!
//! This is timing-sensitive; run repeatedly if investigating a flaky hang.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use h2_support::prelude::*;
use tokio::net::{TcpListener, TcpStream};

static REQUEST_BODY: &[u8] = &[77; 10 * 1024];
static RESPONSE_BODY: &[u8] = &[88; 10 * 1024];

const CONCURRENCY: usize = 50;
const MAX_CONCURRENT_STREAMS: u32 = 10;
const TARGET_REQUESTS_PER_TASK: usize = 40;
const TARGET_REQUESTS_COMPLETED: usize = CONCURRENCY * TARGET_REQUESTS_PER_TASK;
const MUST_MAKE_PROGRESS_INTERVAL: Duration = Duration::from_secs(3);
const CHECK_FOR_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn logical_deadlock_max_concurrent_streams_stress() {
    h2_support::trace_init!();

    let completed = Arc::new(AtomicUsize::new(0));
    let server_addr = spawn_server().await;

    let tcp = TcpStream::connect(server_addr).await.unwrap();
    let (client, h2_connection) = client::handshake(tcp).await.unwrap();

    tokio::spawn(async move {
        if let Err(e) = h2_connection.await {
            // Connection drop at end of test is fine.
            tracing::debug!("h2 connection finished: {:?}", e);
        }
    });

    let mut join_handles = Vec::new();
    for _ in 0..CONCURRENCY {
        let mut client = client.clone();
        let completed = completed.clone();
        join_handles.push(tokio::spawn(async move {
            for _ in 0..TARGET_REQUESTS_PER_TASK {
                // Backpressure when pending_open is full for this SendRequest handle.
                client = client.ready().await.expect("client ready");

                // Absolute URI so :authority is present. After F62 the server
                // rejects scheme+path-only requests with RST PROTOCOL_ERROR.
                let request = Request::builder()
                    .method(Method::POST)
                    .uri("http://localhost/")
                    .body(())
                    .unwrap();

                let (response_future, mut request_body) =
                    client.send_request(request, false).expect("send_request");

                request_body
                    .send_data(Bytes::from_static(REQUEST_BODY), true)
                    .expect("send_data");

                let response = match response_future.await {
                    Ok(response) => response,
                    Err(e) if e.is_reset() && e.reason() == Some(Reason::REFUSED_STREAM) => {
                        completed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    Err(e) => panic!("Request failed unexpectedly: {:?}", e),
                };

                let mut body = response.into_body();
                let mut total_length = 0usize;
                while let Some(chunk) = body.data().await {
                    let chunk = chunk.expect("response body chunk");
                    total_length += chunk.len();
                    body.flow_control()
                        .release_capacity(chunk.len())
                        .expect("release response capacity");
                }
                assert_eq!(total_length, RESPONSE_BODY.len());
                completed.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    let mut last_progress = Instant::now();
    let mut last_completed = 0usize;

    while completed.load(Ordering::Relaxed) < TARGET_REQUESTS_COMPLETED {
        let now_completed = completed.load(Ordering::Relaxed);
        if now_completed > last_completed {
            last_progress = Instant::now();
            last_completed = now_completed;
        }
        if last_progress.elapsed() > MUST_MAKE_PROGRESS_INTERVAL {
            panic!(
                "No requests completed in {:?}; stuck at {}/{} — possible #853 deadlock",
                MUST_MAKE_PROGRESS_INTERVAL, now_completed, TARGET_REQUESTS_COMPLETED
            );
        }
        tokio::time::sleep(CHECK_FOR_PROGRESS_INTERVAL).await;
    }

    for handle in join_handles {
        handle.await.unwrap();
    }
}

async fn spawn_server() -> SocketAddr {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut connection = match server::Builder::new()
                    .max_concurrent_streams(MAX_CONCURRENT_STREAMS)
                    .handshake::<_, Bytes>(socket)
                    .await
                {
                    Ok(c) => c,
                    Err(_) => return,
                };

                while let Some(incoming) = connection.next().await {
                    let Ok((mut request, mut responder)) = incoming else {
                        break;
                    };
                    tokio::spawn(async move {
                        let mut body = request.into_body();
                        let mut total_length = 0usize;
                        while let Some(chunk) = body.data().await {
                            let chunk = chunk.expect("request body");
                            total_length += chunk.len();
                            body.flow_control()
                                .release_capacity(chunk.len())
                                .expect("release request capacity");
                        }
                        assert_eq!(total_length, REQUEST_BODY.len());

                        let response = Response::builder().status(StatusCode::OK).body(()).unwrap();
                        let mut body_sender = responder.send_response(response, false).unwrap();
                        body_sender
                            .send_data(Bytes::from_static(RESPONSE_BODY), true)
                            .unwrap();
                    });
                }
            });
        }
    });

    addr
}
