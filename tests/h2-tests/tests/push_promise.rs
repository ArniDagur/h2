use futures::{StreamExt, TryStreamExt};
use h2_support::prelude::*;
use std::pin::Pin;

#[tokio::test]
async fn recv_push_works() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404)).await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        srv.send_frame(frames::data(2, "promised_data").eos()).await;
    };
    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let pushed = resp.push_promises();
        let check_resp_status = async move {
            let resp = resp.await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        };
        let check_pushed_response = async move {
            let p = pushed.and_then(|headers| async move {
                let (request, response) = headers.into_parts();
                assert_eq!(request.into_parts().0.method, Method::GET);
                let resp = response.await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                let b = util::concat(resp.into_body()).await.unwrap();
                assert_eq!(b, "promised_data");
                Ok(())
            });
            let ps: Vec<_> = p.collect().await;
            assert_eq!(1, ps.len())
        };

        h2.drive(join(check_resp_status, check_pushed_response))
            .await;
    };

    join(mock, h2).await;
}

#[tokio::test]
async fn pushed_streams_arent_dropped_too_early() {
    // tests that by default, received push promises work
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404)).await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 4).request("GET", "https://http2.akamai.com/style2.css"),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
        idle_ms(10).await;
        srv.send_frame(frames::headers(2).response(200)).await;
        srv.send_frame(frames::headers(4).response(200).eos()).await;
        srv.send_frame(frames::data(2, "").eos()).await;
        srv.recv_frame(frames::go_away(4)).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let mut pushed = resp.push_promises();
        let check_status = async move {
            let resp = resp.await.unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        };

        let check_pushed = async move {
            let mut count = 0;
            while let Some(headers) = pushed.next().await {
                let (request, response) = headers.unwrap().into_parts();
                assert_eq!(request.into_parts().0.method, Method::GET);
                let resp = response.await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                count += 1;
            }
            assert_eq!(2, count);
        };

        drop(client);

        h2.drive(join(check_pushed, check_status)).await;
        h2.await.expect("client");
    };

    join(mock, h2).await;
}

#[tokio::test]
async fn recv_push_when_push_disabled_is_conn_error() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let _ = srv.assert_client_handshake().await;
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 3).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(frames::go_away(0).protocol_error()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .enable_push(false)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async move {
            let res = client.send_request(request, true).unwrap().0.await;
            let err = res.unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        // client should see a protocol error
        let conn = async move {
            let res = h2.await;
            let err = res.unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        join(conn, req).await;
    };

    join(mock, h2).await;
}

/// After all PUSH_PROMISEs are delivered, `push_promise()` parks on `push_task`.
/// When the parent stream's response ends receive (HEADERS/DATA EOS), that
/// waiter must be woken so the stream yields `None`. Regression for #811.
#[tokio::test]
async fn push_promises_stream_ends_when_parent_response_finishes() {
    h2_support::trace_init!();
    use std::time::Duration;

    let (io, mut srv) = mock::new();
    let (pushes_done_tx, pushes_done_rx) = tokio::sync::oneshot::channel();
    let (client_done_tx, client_done_rx) = tokio::sync::oneshot::channel();

    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
        )
        .await;
        // Deliver pushed responses so the client can finish each promise
        // before waiting for the parent stream to end the push stream.
        srv.send_frame(frames::headers(2).response(200).eos()).await;
        srv.send_frame(frames::headers(4).response(200).eos()).await;

        // Client has drained both promises and is parked on the next poll.
        pushes_done_rx.await.unwrap();
        // Ending the parent receive side must wake `push_task` (not only
        // `recv_task`). Keep the connection open until the client finishes so
        // EOF cannot spuriously wake the waiter.
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        let _ = client_done_rx.await;
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        // Separate task so missed wakeups hang instead of being polled by drive().
        tokio::spawn(async move {
            let _ = conn.await;
        });

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let mut pushes = resp.push_promises();

        // Collect both push promises first (without ending the parent stream).
        for _ in 0..2 {
            let p = poll_fn(|cx| pushes.poll_push_promise(cx))
                .wakened()
                .await
                .expect("push stream ended early")
                .expect("push error");
            let (req, push_resp) = p.into_parts();
            assert_eq!(req.method(), Method::GET);
            let push_resp = push_resp.await.unwrap();
            assert_eq!(push_resp.status(), StatusCode::OK);
        }

        // Register push_task, then finish the parent response. Without
        // notify_push on recv_headers, this wakened poll never resumes
        // (a plain timeout would re-poll on timer fire and hide the bug).
        let mut end = poll_fn(|cx| pushes.poll_push_promise(cx)).wakened();
        // First poll registers the waker and returns Pending.
        assert!(
            poll_fn(|cx| {
                use std::task::Poll;
                match Pin::new(&mut end).poll(cx) {
                    Poll::Pending => Poll::Ready(false),
                    Poll::Ready(_) => Poll::Ready(true),
                }
            })
            .await
                == false,
            "expected push_promise to park after draining promises"
        );

        let _ = pushes_done_tx.send(());

        let ended = tokio::time::timeout(Duration::from_secs(2), end)
            .await
            .expect("push_promise hung: parent response did not wake push_task (#811)");
        assert!(ended.is_none(), "expected push stream to end with None");

        let resp = resp.await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let _ = client_done_tx.send(());
    };

    join(mock, client).await;
}

#[tokio::test]
async fn pending_push_promises_reset_when_dropped() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(frames::reset(2).cancel()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let req = async {
            let resp = client
                .send_request(request, true)
                .unwrap()
                .0
                .await
                .expect("response");
            assert_eq!(resp.status(), StatusCode::OK);
        };

        conn.drive(req).await;
        conn.await.expect("client");
        drop(client);
    };

    join(srv, client).await;
}

#[tokio::test]
async fn recv_push_promise_over_max_header_list_size() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(64));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.recv_frame(frames::reset(2).protocol_error()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        idle_ms(10).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::Builder::new()
            .max_header_list_size(64)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");
        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async move {
            let resp = client
                .send_request(request, true)
                .expect("send_request")
                .0
                .await
                .expect("response");
            assert_eq!(resp.status(), StatusCode::OK);
        };

        conn.drive(req).await;
        conn.await.expect("client");
    };
    join(srv, client).await;
}

#[tokio::test]
async fn recv_invalid_push_promise_headers_is_stream_protocol_error() {
    // Unsafe method or content length is stream protocol error
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404)).await;
        srv.send_frame(
            frames::push_promise(1, 2).request("POST", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 4)
                .request("GET", "https://http2.akamai.com/style.css")
                .field(http::header::CONTENT_LENGTH, 1),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 6)
                .request("GET", "https://http2.akamai.com/style.css")
                .field(http::header::CONTENT_LENGTH, 0),
        )
        .await;
        srv.send_frame(frames::headers(1).response(404).eos()).await;
        srv.recv_frame(frames::reset(2).protocol_error()).await;
        srv.recv_frame(frames::reset(4).protocol_error()).await;
        srv.send_frame(frames::headers(6).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let (mut resp, _) = client.send_request(request, true).unwrap();
        let check_pushed_response = async move {
            let pushed = resp.push_promises();
            let p = pushed.and_then(|headers| headers.into_parts().1);
            let ps: Vec<_> = p.collect().await;
            // CONTENT_LENGTH = 0 is ok
            assert_eq!(1, ps.len());
        };
        h2.drive(check_pushed_response).await;
    };

    join(mock, h2).await;
}

#[test]
#[ignore]
fn recv_push_promise_with_wrong_authority_is_stream_error() {
    // if server is foo.com, :authority = bar.com is stream error
}

#[tokio::test]
async fn recv_push_promise_skipped_stream_id() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 4).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.recv_frame(frames::go_away(0).protocol_error()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async move {
            let err = client
                .send_request(request, true)
                .unwrap()
                .0
                .await
                .unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        // client should see a protocol error
        let conn = async move {
            let res = h2.await;
            let err = res.unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        join(conn, req).await;
    };

    join(mock, h2).await;
}

#[tokio::test]
async fn recv_push_promise_dup_stream_id() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let mock = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.send_frame(
            frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
        )
        .await;
        srv.recv_frame(frames::go_away(0).protocol_error()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async move {
            let res = client.send_request(request, true).unwrap().0.await;
            let err = res.unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        // client should see a protocol error
        let conn = async move {
            let res = h2.await;
            let err = res.unwrap_err();
            assert_eq!(
                err.to_string(),
                "connection error detected: unspecific protocol error detected"
            );
        };

        join(conn, req).await;
    };

    join(mock, h2).await;
}
