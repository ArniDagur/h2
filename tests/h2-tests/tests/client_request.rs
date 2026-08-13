use futures::future::{ready, Either};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use h2_support::prelude::*;
use std::pin::Pin;
use std::task::Context;
use std::{io, panic};

#[tokio::test]
async fn handshake() {
    h2_support::trace_init!();

    let mock = mock_io::Builder::new()
        .handshake()
        .write(SETTINGS_ACK)
        .build();

    let (_client, h2) = client::handshake(mock).await.unwrap();

    tracing::trace!("hands have been shook");

    // At this point, the connection should be closed
    h2.await.unwrap();
}

#[tokio::test]
async fn client_other_thread() {
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
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let request = Request::builder()
                .uri("https://http2.akamai.com/")
                .body(())
                .unwrap();
            let _res = client
                .send_request(request, true)
                .unwrap()
                .0
                .await
                .expect("request");
        });
        h2.await.expect("h2");
    };
    join(srv, h2).await;
}

#[tokio::test]
async fn recv_invalid_server_stream_id() {
    h2_support::trace_init!();

    let mock = mock_io::Builder::new()
        .handshake()
        // Write GET /
        .write(&[
            0, 0, 0x10, 1, 5, 0, 0, 0, 1, 0x82, 0x87, 0x41, 0x8B, 0x9D, 0x29, 0xAC, 0x4B, 0x8F,
            0xA8, 0xE9, 0x19, 0x97, 0x21, 0xE9, 0x84,
        ])
        .write(SETTINGS_ACK)
        // Read response
        .read(&[0, 0, 1, 1, 5, 0, 0, 0, 2, 137])
        // Write GO_AWAY
        .write(&[0, 0, 8, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
        .build();

    let (mut client, h2) = client::handshake(mock).await.unwrap();

    // Send the request
    let request = Request::builder()
        .uri("https://http2.akamai.com/")
        .body(())
        .unwrap();

    tracing::info!("sending request");
    let (response, _) = client.send_request(request, true).unwrap();

    // The connection errors
    assert!(h2.await.is_err());

    // The stream errors
    assert!(response.await.is_err());
}

#[tokio::test]
async fn request_stream_id_overflows() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let h2 = async move {
        let (mut client, mut h2) = client::Builder::new()
            .initial_stream_id(u32::MAX >> 1)
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (response, _) = client.send_request(request, true).unwrap();
        let _x = h2.drive(response).await.unwrap();

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        // second cannot use the next stream id, it's over
        let poll_err = poll_fn(|cx| client.poll_ready(cx)).await.unwrap_err();
        assert_eq!(poll_err.to_string(), "user error: stream ID overflowed");

        let err = client.send_request(request, true).unwrap_err();
        assert_eq!(err.to_string(), "user error: stream ID overflowed");

        h2.await.unwrap();
    };

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(u32::MAX >> 1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(u32::MAX >> 1).response(200).eos())
            .await;
        idle_ms(10).await;
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn client_builder_max_concurrent_streams() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let mut settings = frame::Settings::default();
    settings.set_max_concurrent_streams(Some(1));

    let srv = async move {
        let rcvd_settings = srv.assert_client_handshake().await;
        assert_frame_eq(settings, rcvd_settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let mut builder = client::Builder::new();
    builder.max_concurrent_streams(1);

    let h2 = async move {
        let (mut client, mut h2) = builder.handshake::<_, Bytes>(io).await.unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn request_over_max_concurrent_streams_errors() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(
                frames::settings()
                    // super tiny server
                    .max_concurrent_streams(1),
            )
            .await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(frames::headers(3).request("POST", "https://example.com/"))
            .await;
        srv.send_frame(frames::headers(3).response(200)).await;
        srv.recv_frame(frames::data(3, "hello").eos()).await;
        srv.send_frame(frames::data(3, "").eos()).await;
        srv.recv_frame(frames::headers(5).request("POST", "https://example.com/"))
            .await;
        srv.send_frame(frames::headers(5).response(200)).await;
        srv.recv_frame(frames::data(5, "hello").eos()).await;
        srv.send_frame(frames::data(5, "").eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.expect("handshake");
        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        // first request is allowed
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (resp1, mut stream1) = client.send_request(request, false).unwrap();
        // as long as we let the connection internals tick
        client = h2.drive(client.ready()).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // second request is put into pending_open
        let (resp2, mut stream2) = client.send_request(request, false).unwrap();

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        // third stream is over max concurrent
        assert!(!client.poll_ready(&mut cx).is_ready());

        let err = client.send_request(request, true).unwrap_err();
        assert_eq!(err.to_string(), "user error: rejected");

        stream1
            .send_data("hello".into(), true)
            .expect("req send_data");

        h2.drive(async move {
            resp1.await.expect("req");
            stream2
                .send_data("hello".into(), true)
                .expect("req2 send_data");
        })
        .await;
        join(async move { h2.await.unwrap() }, async move {
            resp2.await.unwrap()
        })
        .await;
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn recv_decrement_max_concurrent_streams_when_requests_queued() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;

        srv.ping_pong([0; 8]).await;

        // limit this server later in life
        srv.send_frame(frames::settings().max_concurrent_streams(1))
            .await;
        srv.recv_frame(frames::settings_ack()).await;
        srv.recv_frame(
            frames::headers(3)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.ping_pong([1; 8]).await;
        srv.send_frame(frames::headers(3).response(200).eos()).await;

        srv.recv_frame(
            frames::headers(5)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(5).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.expect("handshake");
        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        // first request is allowed
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (resp1, _) = client.send_request(request, true).unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // second request is put into pending_open
        let (resp2, _) = client.send_request(request, true).unwrap();

        h2.drive(async move {
            resp1.await.expect("req");
        })
        .await;
        join(async move { h2.await.unwrap() }, async move {
            resp2.await.unwrap()
        })
        .await;
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn send_request_poll_ready_when_connection_error() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(
                frames::settings()
                    // super tiny server
                    .max_concurrent_streams(1),
            )
            .await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(
            frames::headers(3)
                .request("POST", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(8).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.expect("handshake");
        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (resp1, _) = client.send_request(request, true).unwrap();
        // as long as we let the connection internals tick
        client = h2.drive(client.ready()).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // second request is put into pending_open
        let (resp2, _) = client.send_request(request, true).unwrap();

        // third stream is over max concurrent
        let until_ready = async move {
            poll_fn(move |cx| client.poll_ready(cx))
                .await
                .expect_err("client poll_ready");
        };

        // a FuturesUnordered is used on purpose!
        //
        // We don't want a join, since any of the other futures notifying
        // will make the until_ready future polled again, but we are
        // specifically testing that until_ready gets notified on its own.
        let mut unordered =
            futures::stream::FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();
        unordered.push(Box::pin(until_ready));
        unordered.push(Box::pin(async move {
            h2.await.expect_err("client conn");
        }));
        unordered.push(Box::pin(async move {
            resp1.await.expect_err("req1");
        }));
        unordered.push(Box::pin(async move {
            resp2.await.expect_err("req2");
        }));

        while unordered.next().await.is_some() {}
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn send_reset_notifies_recv_stream() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.recv_frame(frames::reset(1).refused()).await;
        srv.recv_frame(frames::go_away(0)).await;
        srv.recv_eof().await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.expect("handshake");
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // first request is allowed
        let (resp1, mut tx) = client.send_request(request, false).unwrap();
        let res = conn.drive(resp1).await.unwrap();

        let tx = async move {
            tx.send_reset(h2::Reason::REFUSED_STREAM);
        };
        let rx = async {
            let mut body = res.into_body();
            let err = body.next().await.unwrap().expect_err("RecvBody");
            assert_eq!(
                err.to_string(),
                "stream error sent by user: refused stream before processing any application logic"
            );
        };

        // a FuturesUnordered is used on purpose!
        //
        // We don't want a join, since any of the other futures notifying
        // will make the rx future polled again, but we are
        // specifically testing that rx gets notified on its own.
        let unordered = FuturesUnordered::<Pin<Box<dyn Future<Output = ()>>>>::new();
        unordered.push(Box::pin(rx));
        unordered.push(Box::pin(tx));

        conn.drive(unordered.for_each(ready)).await;
        drop(client); // now let client gracefully goaway
        conn.await.expect("client");
    };

    join(srv, client).await;
}

#[tokio::test]
async fn http_11_request_without_scheme_or_authority() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("GET", "/").scheme("http").eos())
            .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.expect("handshake");

        // HTTP_11 request with just :path is allowed
        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(())
            .unwrap();

        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn http_2_request_without_scheme_or_authority() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.expect("handshake");

        // HTTP_2 with only a :path is illegal, so this request should
        // be rejected as a user error.
        let request = Request::builder()
            .version(Version::HTTP_2)
            .method(Method::GET)
            .uri("/")
            .body(())
            .unwrap();

        client
            .send_request(request, true)
            .expect_err("should be UserError");
        let _: () = h2.await.expect("h2");
        drop(client);
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn http_2_connect_request_omit_scheme_and_path_fields() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .pseudo(frame::Pseudo {
                    method: Method::CONNECT.into(),
                    authority: util::byte_str("tunnel.example.com:8443").into(),
                    ..Default::default()
                })
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.expect("handshake");

        // In HTTP_2 CONNECT request the ":scheme" and ":path" pseudo-header fields MUST be omitted.
        let request = Request::builder()
            .version(Version::HTTP_2)
            .method(Method::CONNECT)
            .uri("https://tunnel.example.com:8443/")
            .body(())
            .unwrap();

        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();
    };

    join(srv, h2).await;
}

#[test]
#[ignore]
fn request_with_h1_version() {}

#[tokio::test]
async fn request_with_connection_headers() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    // can't assert full handshake, since client never sends a request, and
    // thus never bothers to ack the settings...
    let srv = async move {
        srv.read_preface().await.unwrap();
        srv.recv_frame(frames::settings()).await;
        // goaway is required to make sure the connection closes because
        // of no active streams
        srv.recv_frame(frames::go_away(0)).await;
    };

    let headers = vec![
        ("connection", "foo"),
        ("keep-alive", "5"),
        ("proxy-connection", "bar"),
        ("transfer-encoding", "chunked"),
        ("upgrade", "HTTP/2"),
        ("te", "boom"),
    ];

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.expect("handshake");

        for (name, val) in headers {
            let req = Request::builder()
                .uri("https://http2.akamai.com/")
                .header(name, val)
                .body(())
                .unwrap();
            let err = client.send_request(req, true).expect_err(name);
            assert_eq!(err.to_string(), "user error: malformed headers");
        }
        drop(client);
        conn.await.unwrap();
    };

    join(srv, client).await;
}

#[tokio::test]
async fn connection_close_notifies_response_future() {
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
        // don't send any response, just close
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.expect("handshake");

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async move {
            let res = client
                .send_request(request, true)
                .expect("send_request1")
                .0
                .await;
            let err = res.expect_err("response");
            assert_eq!(err.to_string(), "stream closed because of a broken pipe");
        };
        join(async move { conn.await.expect("conn") }, req).await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn connection_close_notifies_client_poll_ready() {
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
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.expect("handshake");

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req = async {
            let res = client
                .send_request(request, true)
                .expect("send_request1")
                .0
                .await;
            let err = res.expect_err("response");
            assert_eq!(err.to_string(), "stream closed because of a broken pipe");
        };

        conn.drive(req).await;

        let err = poll_fn(move |cx| client.poll_ready(cx))
            .await
            .expect_err("poll_ready");
        assert_eq!(
            err.to_string(),
            "connection closed because of a broken pipe"
        );
    };

    join(srv, client).await;
}

#[tokio::test]
async fn sending_request_on_closed_connection() {
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
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        // a bad frame!
        srv.send_frame(frames::headers(0).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.expect("handshake");

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        // first request works
        let req = Box::pin(async {
            client
                .send_request(request, true)
                .expect("send_request1")
                .0
                .await
                .expect("response1");
        });

        // after finish request1, there should be a conn error
        let h2 = Box::pin(async move {
            h2.await.expect_err("h2 error");
        });

        match select(h2, req).await {
            Either::Left((_, req)) => req.await,
            Either::Right((_, _h2)) => unreachable!("Shouldn't happen"), // TODO: Is this correct?
        };

        let poll_err = poll_fn(|cx| client.poll_ready(cx)).await.unwrap_err();
        let msg = "connection error detected: unspecific protocol error detected";
        assert_eq!(poll_err.to_string(), msg);

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();
        let send_err = client.send_request(request, true).unwrap_err();
        assert_eq!(send_err.to_string(), msg);
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn recv_too_big_headers() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(40));
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.recv_frame(
            frames::headers(3)
                .request("GET", "https://http2.akamai.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.send_frame(frames::headers(3).response(200)).await;
        // no reset for 1, since it's closed anyway
        // but reset for 3, since server hasn't closed stream
        srv.recv_frame(frames::reset(3).protocol_error()).await;
        idle_ms(10).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::Builder::new()
            .max_header_list_size(40)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req1 = client.send_request(request, true);
        // Spawn tasks to ensure that the error wakes up tasks that are blocked
        // waiting for a response.
        let req1 = async move {
            let err = req1.expect("send_request").0.await.expect_err("response1");
            assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
        };

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let req2 = client.send_request(request, true);
        let req2 = async move {
            let err = req2.expect("send_request").0.await.expect_err("response2");
            assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
        };

        conn.drive(join(req1, req2)).await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn pending_send_request_gets_reset_by_peer_properly() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let payload = Bytes::from(vec![0; (frame::DEFAULT_INITIAL_WINDOW_SIZE * 2) as usize]);
    let max_frame_size = frame::DEFAULT_MAX_FRAME_SIZE as usize;

    let srv = async {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("GET", "https://http2.akamai.com/"))
            .await;
        // Note that we can only send up to ~4 frames of data by default
        srv.recv_frame(frames::data(1, &payload[0..max_frame_size]))
            .await;
        srv.recv_frame(frames::data(
            1,
            &payload[max_frame_size..(max_frame_size * 2)],
        ))
        .await;
        srv.recv_frame(frames::data(
            1,
            &payload[(max_frame_size * 2)..(max_frame_size * 3)],
        ))
        .await;
        srv.recv_frame(frames::data(
            1,
            &payload[(max_frame_size * 3)..(max_frame_size * 4 - 1)],
        ))
        .await;

        idle_ms(100).await;

        srv.send_frame(frames::reset(1).refused()).await;
        // Because all active requests are finished, connection should shutdown
        // and send a GO_AWAY frame. If the reset stream is bugged (and doesn't
        // count towards concurrency limit), then connection will not send
        // a GO_AWAY and this test will fail.
        srv.recv_frame(frames::go_away(0)).await;
        drop(srv);
    };

    let client = async {
        let (mut client, mut conn) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let request = Request::builder()
            .uri("https://http2.akamai.com/")
            .body(())
            .unwrap();

        let (response, mut stream) = client.send_request(request, false).expect("send_request");

        let response = async move {
            let err = response.await.expect_err("response");
            assert_eq!(err.reason(), Some(Reason::REFUSED_STREAM));
        };

        // Send the data
        stream.send_data(payload.clone(), true).unwrap();
        conn.drive(response).await;
        drop(client);
        drop(stream);
        conn.await.expect("client");
    };

    join(srv, client).await;
}

#[tokio::test]
async fn request_without_path() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "http://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.expect("handshake");
        // Note the lack of trailing slash.
        let request = Request::get("http://example.com").body(()).unwrap();

        let (response, _) = client.send_request(request, true).unwrap();

        conn.drive(response).await.unwrap();
    };

    join(srv, client).await;
}

#[tokio::test]
async fn request_options_with_star() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    // Note the lack of trailing slash.
    let uri = uri::Uri::from_parts({
        let mut parts = uri::Parts::default();
        parts.scheme = Some(uri::Scheme::HTTP);
        parts.authority = Some(uri::Authority::from_static("example.com"));
        parts.path_and_query = Some(uri::PathAndQuery::from_static("*"));
        parts
    })
    .unwrap();

    let uri_clone = uri.clone();
    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("OPTIONS", uri_clone).eos())
            .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.expect("handshake");
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri(uri)
            .body(())
            .unwrap();

        let (response, _) = client.send_request(request, true).unwrap();

        conn.drive(response).await.unwrap();
    };

    join(srv, client).await;
}

#[tokio::test]
async fn notify_on_send_capacity() {
    // This test ensures that the client gets notified when there is additional
    // send capacity. In other words, when the server is ready to accept a new
    // stream, the client is notified.
    use tokio::sync::oneshot;

    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let (done_tx, done_rx) = oneshot::channel();
    let (tx, rx) = oneshot::channel();

    let mut settings = frame::Settings::default();
    settings.set_max_concurrent_streams(Some(1));

    let srv = async move {
        let settings = srv.assert_client_handshake_with_settings(settings).await;
        // This is the ACK
        assert_default_settings!(settings);
        tx.send(()).unwrap();
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://www.example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(
            frames::headers(3)
                .request("GET", "https://www.example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(3).response(200).eos()).await;
        srv.recv_frame(
            frames::headers(5)
                .request("GET", "https://www.example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(5).response(200).eos()).await;
        // Don't close the connection until the client is done doing its
        // checks.
        done_rx.await.unwrap();
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.expect("handshake");
        tokio::spawn(async move {
            rx.await.unwrap();

            let mut responses = vec![];

            for _ in 0..3usize {
                // Wait for capacity. If the client is **not** notified,
                // this hangs.
                poll_fn(|cx| client.poll_ready(cx)).await.unwrap();

                let request = Request::builder()
                    .uri("https://www.example.com/")
                    .body(())
                    .unwrap();

                let response = client.send_request(request, true).unwrap().0;

                responses.push(response);
            }

            for response in responses {
                let response = response.await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }

            poll_fn(|cx| client.poll_ready(cx)).await.unwrap();

            done_tx.send(()).unwrap();
        });

        conn.await.expect("h2");
    };

    join(srv, client).await;
}

#[tokio::test]
async fn send_stream_poll_reset() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        srv.send_frame(frames::reset(1).refused()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        let (_response, mut tx) = client.send_request(request, false).unwrap();
        let reason = conn
            .drive(poll_fn(move |cx| tx.poll_reset(cx)))
            .await
            .unwrap();
        assert_eq!(reason, Reason::REFUSED_STREAM);
    };

    join(srv, client).await;
}

#[tokio::test]
async fn drop_pending_open() {
    // This test checks that a stream queued for pending open behaves correctly when its
    // client drops.
    use tokio::sync::oneshot;
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let (init_tx, init_rx) = oneshot::channel();
    let (trigger_go_away_tx, trigger_go_away_rx) = oneshot::channel();
    let (sent_go_away_tx, sent_go_away_rx) = oneshot::channel();
    let (drop_tx, drop_rx) = oneshot::channel();

    let mut settings = frame::Settings::default();
    settings.set_max_concurrent_streams(Some(2));

    let srv = async move {
        let settings = srv.assert_client_handshake_with_settings(settings).await;
        // This is the ACK
        assert_default_settings!(settings);
        init_tx.send(()).unwrap();
        srv.recv_frame(frames::headers(1).request("GET", "https://www.example.com/"))
            .await;
        srv.recv_frame(
            frames::headers(3)
                .request("GET", "https://www.example.com/")
                .eos(),
        )
        .await;
        trigger_go_away_rx.await.unwrap();
        srv.send_frame(frames::go_away(3)).await;
        sent_go_away_tx.send(()).unwrap();
        drop_rx.await.unwrap();
        srv.send_frame(frames::headers(3).response(200).eos()).await;
        srv.recv_frame(frames::data(1, vec![]).eos()).await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    fn request() -> Request<()> {
        Request::builder()
            .uri("https://www.example.com/")
            .body(())
            .unwrap()
    }

    let client = async move {
        let (mut client, conn) = client::Builder::new()
            .max_concurrent_reset_streams(0)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");
        let f = async move {
            init_rx.await.expect("init_rx");
            // Fill up the concurrent stream limit.
            poll_fn(|cx| client.poll_ready(cx)).await.unwrap();
            let mut response1 = client.send_request(request(), false).unwrap();
            poll_fn(|cx| client.poll_ready(cx)).await.unwrap();
            let response2 = client.send_request(request(), true).unwrap();
            poll_fn(|cx| client.poll_ready(cx)).await.unwrap();
            let response3 = client.send_request(request(), true).unwrap();

            // Trigger a GOAWAY frame to invalidate our third request.
            trigger_go_away_tx.send(()).unwrap();
            sent_go_away_rx.await.expect("sent_go_away_rx");
            // Now drop all the references to that stream.
            drop(response3);
            drop(client);
            drop_tx.send(()).unwrap();

            // Complete the second request, freeing up a stream.
            response2.0.await.expect("resp2");
            response1.1.send_data(Default::default(), true).unwrap();
            response1.0.await.expect("resp1")
        };

        join(
            async move {
                conn.await.expect("h2");
            },
            f,
        )
        .await;
    };

    join(srv, client).await;
}

/// Flooding `send_request` before streams leave `pending_open` must engage
/// per-handle backpressure once open+pending_open reaches max concurrent.
#[tokio::test]
async fn pending_open_counts_toward_send_capacity_backpressure() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(2))
            .await;
        assert_default_settings!(settings);
        // Warm-up + two queued streams.
        for id in [1u32, 3, 5] {
            srv.recv_frame(
                frames::headers(id)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
            srv.send_frame(frames::headers(id).response(200).eos()).await;
        }
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        let req = || {
            Request::builder()
                .uri("https://example.com/")
                .body(())
                .unwrap()
        };

        // Drive one request so peer MAX_CONCURRENT_STREAMS is applied.
        let (warm, _) = client.send_request(req(), true).unwrap();
        conn.drive(warm).await.unwrap();
        assert_eq!(conn.max_concurrent_send_streams(), 2);

        // Queue two more without driving: both remain pending_open.
        // Occupancy (0 open + 2 pending) == max; further send_request is rejected.
        let (r1, _) = client.send_request(req(), true).unwrap();
        let (r2, _) = client.send_request(req(), true).unwrap();
        let err = client.send_request(req(), true).unwrap_err();
        assert_eq!(err.to_string(), "user error: rejected");

        conn.drive(async {
            assert_eq!(r1.await.unwrap().status(), StatusCode::OK);
            assert_eq!(r2.await.unwrap().status(), StatusCode::OK);
        })
        .await;
        drop(client);
        conn.await.unwrap();
    };

    join(srv, client).await;
}

/// `SendRequest::poll_ready` (pending_open) and `SendStream::poll_capacity`
/// used to share a single `send_task` waker slot. Concurrent waiters lost
/// wakeups: capacity registration stole the ready waker, so after a concurrent
/// stream slot freed, `poll_ready` never resumed.
#[tokio::test]
async fn pending_open_ready_not_stolen_by_poll_capacity() {
    h2_support::trace_init!();
    use std::time::Duration;
    use tokio::sync::oneshot;

    let (io, mut srv) = mock::new();
    let (release_tx, release_rx) = oneshot::channel();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(
                frames::settings().max_concurrent_streams(1),
            )
            .await;
        assert_default_settings!(settings);

        // Warm-up request so the client applies SETTINGS before the race.
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/warmup")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;

        // Slot holder.
        srv.recv_frame(
            frames::headers(3)
                .request("GET", "https://example.com/hold")
                .eos(),
        )
        .await;
        release_rx.await.unwrap();
        srv.send_frame(frames::headers(3).response(200).eos()).await;

        // Pending-open stream after hold frees the slot.
        srv.recv_frame(frames::headers(5).request("POST", "https://example.com/body"))
            .await;
        srv.recv_frame(frames::data(5, "hi").eos()).await;
        srv.send_frame(frames::headers(5).response(200).eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        // Drive SETTINGS + warm-up so max_concurrent_streams=1 is active.
        let warmup = Request::builder()
            .uri("https://example.com/warmup")
            .body(())
            .unwrap();
        let (warmup_resp, _) = client.send_request(warmup, true).unwrap();
        conn.drive(warmup_resp).await.unwrap();

        // Fill the single concurrent slot.
        let hold = Request::builder()
            .uri("https://example.com/hold")
            .body(())
            .unwrap();
        let (hold_resp, _) = client.send_request(hold, true).unwrap();
        // Tick so the hold stream is counted against max concurrent.
        client = conn.drive(client.ready()).await.unwrap();

        // This stream stays pending_open; original SendRequest keeps `pending`
        // (Clone clears it).
        let body_req = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/body")
            .body(())
            .unwrap();
        let (body_resp, mut send_body) = client.send_request(body_req, false).unwrap();
        send_body.reserve_capacity(2);

        // Connection on a separate task so missed wakeups hang instead of
        // progressing via drive().
        tokio::spawn(async move {
            let _ = conn.await;
        });

        // Park ready() first (open_task), then poll_capacity (send_task).
        let ready_handle = tokio::spawn(async move {
            client.ready().await.expect("ready after open")
        });
        tokio::task::yield_now().await;

        let (cap_registered_tx, cap_registered_rx) = oneshot::channel();
        let cap_handle = tokio::spawn(async move {
            let mut registered = Some(cap_registered_tx);
            let cap = poll_fn(|cx| {
                let poll = send_body.poll_capacity(cx);
                if let Some(tx) = registered.take() {
                    let _ = tx.send(());
                }
                poll
            })
            .await
            .expect("capacity ended")
            .expect("capacity err");
            send_body
                .send_data(Bytes::from_static(b"hi"), true)
                .expect("send_data");
            cap
        });
        cap_registered_rx.await.expect("cap registered");

        // Free the concurrent slot so the pending stream can open.
        let _ = release_tx.send(());
        let _ = hold_resp.await;

        let _client = tokio::time::timeout(Duration::from_secs(2), ready_handle)
            .await
            .expect("SendRequest::ready hung (open waker stolen by poll_capacity)")
            .expect("ready task join");

        let cap = tokio::time::timeout(Duration::from_secs(2), cap_handle)
            .await
            .expect("poll_capacity hung")
            .expect("cap task join");
        assert!(cap > 0);

        let _ = body_resp.await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn malformed_response_headers_dont_unlink_stream() {
    // This test checks that receiving malformed headers frame on a stream with
    // no remaining references correctly resets the stream, without prematurely
    // unlinking it.
    use tokio::sync::oneshot;
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();
    let (drop_tx, drop_rx) = oneshot::channel();
    let (queued_tx, queued_rx) = oneshot::channel();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);

        srv.recv_frame(frames::headers(1).request("GET", "http://example.com/"))
            .await;
        srv.recv_frame(frames::headers(3).request("GET", "http://example.com/"))
            .await;
        srv.recv_frame(frames::headers(5).request("GET", "http://example.com/"))
            .await;
        drop_tx.send(()).unwrap();
        queued_rx.await.unwrap();
        srv.send_bytes(&[
            // 2 byte frame
            0, 0, 2, // type: HEADERS
            1, // flags: END_STREAM | END_HEADERS
            5, // stream identifier: 3
            0, 0, 0, 3, // data - invalid (pseudo not at end of block)
            144,
            135, // Per the spec, this frame should cause a stream error of type
                 // PROTOCOL_ERROR.
        ])
        .await;
    };

    fn request() -> Request<()> {
        Request::builder()
            .uri("http://example.com/")
            .body(())
            .unwrap()
    }

    let client = async move {
        let (mut client, conn) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let (_req1, mut send1) = client.send_request(request(), false).unwrap();
        // Use up most of the connection window.
        send1.send_data(vec![0; 65534].into(), true).unwrap();
        let (req2, mut send2) = client.send_request(request(), false).unwrap();
        let (req3, mut send3) = client.send_request(request(), false).unwrap();

        let f = async move {
            drop_rx.await.unwrap();
            // Use up the remainder of the connection window.
            send2.send_data(vec![0; 2].into(), true).unwrap();
            // Queue up for more connection window.
            send3.send_data(vec![0; 1].into(), true).unwrap();
            queued_tx.send(()).unwrap();
            drop((req2, req3));
        };

        join(async move { conn.await.expect("h2") }, f).await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn allow_empty_data_for_head() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("HEAD", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(200)
                .field("content-length", 100),
        )
        .await;
        srv.send_frame(frames::data(1, "").eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::HEAD)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let (_, mut body) = response.await.unwrap().into_parts();
        assert_eq!(body.data().await.unwrap().unwrap(), "");
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn reject_none_zero_content_length_header_with_end_stream() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(
            frames::headers(1)
                .response(200)
                .field("content-length", 100)
                .eos(),
        )
        .await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let _ = response.await.unwrap_err();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn early_hints() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(103)).await;
        srv.send_frame(frames::headers(1).response(200).field("content-length", 2))
            .await;
        srv.send_frame(frames::data(1, "ok").eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        let (ha, mut body) = response.await.unwrap().into_parts();
        eprintln!("{:?}", ha);
        assert_eq!(body.data().await.unwrap().unwrap(), "ok");
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn informational_while_local_streaming() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.recv_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        srv.send_frame(frames::headers(1).response(103)).await;
        srv.send_frame(frames::headers(1).response(200).field("content-length", 2))
            .await;
        srv.recv_frame(frames::data(1, "hello").eos()).await;
        srv.send_frame(frames::data(1, "ok").eos()).await;
    };

    let h2 = async move {
        let (mut client, h2) = client::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .unwrap();
        tokio::spawn(async {
            h2.await.expect("connection failed");
        });
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();
        // don't EOS stream yet..
        let (response, mut body_tx) = client.send_request(request, false).unwrap();
        // eventual response is 200, not 103
        let resp = response.await.expect("response");
        // assert_eq!(resp.status(), 200);
        // now we can end the stream
        body_tx.send_data("hello".into(), true).expect("send_data");
        let mut body = resp.into_body();
        assert_eq!(body.data().await.unwrap().unwrap(), "ok");
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn extended_connect_protocol_disabled_by_default() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();

        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::get("https://example.com/").body(()).unwrap();
        // first request is allowed
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();

        assert!(!client.is_extended_connect_protocol_enabled());
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn extended_connect_protocol_enabled_during_handshake() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().enable_connect_protocol(1))
            .await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();

        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::get("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();

        assert!(client.is_extended_connect_protocol_enabled());
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn invalid_connect_protocol_enabled_setting() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();

    let srv = async move {
        // Send a settings frame
        srv.send(frames::settings().enable_connect_protocol(2).into())
            .await
            .unwrap();
        srv.read_preface().await.unwrap();

        let settings = assert_settings!(srv.next().await.expect("unexpected EOF").unwrap());
        assert_default_settings!(settings);

        // Send the ACK
        let ack = frame::Settings::ack();

        // TODO: Don't unwrap?
        srv.send(ack.into()).await.unwrap();

        let frame = srv.next().await.unwrap().unwrap();
        let go_away = assert_go_away!(frame);
        assert_eq!(go_away.reason(), Reason::PROTOCOL_ERROR);
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();

        // we send a simple req here just to drive the connection so we can
        // receive the server settings.
        let request = Request::get("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(request, true).unwrap();

        let error = h2.drive(response).await.unwrap_err();
        assert_eq!(error.reason(), Some(Reason::PROTOCOL_ERROR));
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn extended_connect_request() {
    h2_support::trace_init!();

    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().enable_connect_protocol(1))
            .await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .pseudo(frame::Pseudo {
                    method: Method::CONNECT.into(),
                    scheme: util::byte_str("http").into(),
                    authority: util::byte_str("bread").into(),
                    path: util::byte_str("/baguette").into(),
                    protocol: Protocol::from_static("the-bread-protocol").into(),
                    ..Default::default()
                })
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();

        let request = Request::connect("http://bread/baguette")
            .extension(Protocol::from("the-bread-protocol"))
            .body(())
            .unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn rogue_server_odd_headers() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.send_frame(frames::headers(1)).await;
        srv.recv_frame(frames::go_away(0).protocol_error()).await;
    };

    let h2 = async move {
        let (_client, h2) = client::handshake(io).await.unwrap();

        let err = h2.await.unwrap_err();
        assert!(err.is_go_away());
        assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn rogue_server_even_headers() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);
        srv.send_frame(frames::headers(2)).await;
        srv.recv_frame(frames::go_away(0).protocol_error()).await;
    };

    let h2 = async move {
        let (_client, h2) = client::handshake(io).await.unwrap();

        let err = h2.await.unwrap_err();
        assert!(err.is_go_away());
        assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn rogue_server_reused_headers() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://camembert.fromage")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.send_frame(frames::headers(1)).await;
        srv.recv_frame(frames::reset(1).stream_closed()).await;
    };

    let h2 = async move {
        let (mut client, mut h2) = client::handshake(io).await.unwrap();

        h2.drive(async {
            let request = Request::builder()
                .method(Method::GET)
                .uri("https://camembert.fromage")
                .body(())
                .unwrap();
            let _res = client.send_request(request, true).unwrap().0.await.unwrap();
        })
        .await;

        h2.await.unwrap();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn client_builder_header_table_size() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();
    let mut settings = frame::Settings::default();

    settings.set_header_table_size(Some(10000));

    let srv = async move {
        let recv_settings = srv.assert_client_handshake().await;
        assert_frame_eq(recv_settings, settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(1).response(200).eos()).await;
    };

    let mut builder = client::Builder::new();
    builder.header_table_size(10000);

    let h2 = async move {
        let (mut client, mut h2) = builder.handshake::<_, Bytes>(io).await.unwrap();
        let request = Request::get("https://example.com/").body(()).unwrap();
        let (response, _) = client.send_request(request, true).unwrap();
        h2.drive(response).await.unwrap();
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn configured_max_concurrent_send_streams_and_update_it_based_on_empty_settings_frame() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        // Send empty SETTINGS frame (no MAX_CONCURRENT_STREAMS is provided)
        srv.send_frame(frames::settings()).await;
    };

    let h2 = async move {
        let (_client, h2) = client::Builder::new()
            // Configure the initial value to 2024
            .initial_max_send_streams(2024)
            .handshake::<_, bytes::Bytes>(io)
            .await
            .unwrap();
        let mut h2 = std::pin::pin!(h2);
        // It should be pre-configured value before it receives the initial
        // SETTINGS frame from the server
        assert_eq!(h2.max_concurrent_send_streams(), 2024);
        h2.as_mut().await.unwrap();
        // If the server's initial SETTINGS frame does not include
        // MAX_CONCURRENT_STREAMS, this should be updated to usize::MAX.
        assert_eq!(h2.max_concurrent_send_streams(), usize::MAX);
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn configured_max_concurrent_send_streams_and_update_it_based_on_non_empty_settings_frame() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        // Send SETTINGS frame with MAX_CONCURRENT_STREAMS set to 42
        srv.send_frame(frames::settings().max_concurrent_streams(42))
            .await;
    };

    let h2 = async move {
        let (_client, h2) = client::Builder::new()
            // Configure the initial value to 2024
            .initial_max_send_streams(2024)
            .handshake::<_, bytes::Bytes>(io)
            .await
            .unwrap();
        let mut h2 = std::pin::pin!(h2);
        // It should be pre-configured value before it receives the initial
        // SETTINGS frame from the server
        assert_eq!(h2.max_concurrent_send_streams(), 2024);
        h2.as_mut().await.unwrap();
        // Now the client has received the initial SETTINGS frame from the
        // server, which should update the value accordingly
        assert_eq!(h2.max_concurrent_send_streams(), 42);
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn receive_settings_frame_twice_with_second_one_empty() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        // Send the initial SETTINGS frame with MAX_CONCURRENT_STREAMS set to 42
        srv.send_frame(frames::settings().max_concurrent_streams(42))
            .await;

        // Handle the client's connection preface
        srv.read_preface().await.unwrap();
        match srv.next().await {
            Some(frame) => match frame.unwrap() {
                h2::frame::Frame::Settings(_) => {
                    let ack = frame::Settings::ack();
                    srv.send(ack.into()).await.unwrap();
                }
                frame => {
                    panic!("unexpected frame: {:?}", frame);
                }
            },
            None => {
                panic!("unexpected EOF");
            }
        }

        // Should receive the ack for the server's initial SETTINGS frame
        let frame = assert_settings!(srv.next().await.unwrap().unwrap());
        assert!(frame.is_ack());

        // Send another SETTINGS frame with no MAX_CONCURRENT_STREAMS
        // This should not update the max_concurrent_send_streams value that
        // the client manages.
        srv.send_frame(frames::settings()).await;
    };

    let h2 = async move {
        let (_client, h2) = client::handshake(io).await.unwrap();
        let mut h2 = std::pin::pin!(h2);
        assert_eq!(h2.max_concurrent_send_streams(), usize::MAX);
        h2.as_mut().await.unwrap();
        // Even though the second SETTINGS frame contained no value for
        // MAX_CONCURRENT_STREAMS, update to usize::MAX should not happen
        assert_eq!(h2.max_concurrent_send_streams(), 42);
    };

    join(srv, h2).await;
}

#[tokio::test]
async fn receive_settings_frame_twice_with_second_one_non_empty() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        // Send the initial SETTINGS frame with MAX_CONCURRENT_STREAMS set to 42
        srv.send_frame(frames::settings().max_concurrent_streams(42))
            .await;

        // Handle the client's connection preface
        srv.read_preface().await.unwrap();
        match srv.next().await {
            Some(frame) => match frame.unwrap() {
                h2::frame::Frame::Settings(_) => {
                    let ack = frame::Settings::ack();
                    srv.send(ack.into()).await.unwrap();
                }
                frame => {
                    panic!("unexpected frame: {:?}", frame);
                }
            },
            None => {
                panic!("unexpected EOF");
            }
        }

        // Should receive the ack for the server's initial SETTINGS frame
        let frame = assert_settings!(srv.next().await.unwrap().unwrap());
        assert!(frame.is_ack());

        // Send another SETTINGS frame with no MAX_CONCURRENT_STREAMS
        // This should not update the max_concurrent_send_streams value that
        // the client manages.
        srv.send_frame(frames::settings().max_concurrent_streams(2024))
            .await;
    };

    let h2 = async move {
        let (_client, h2) = client::handshake(io).await.unwrap();
        let mut h2 = std::pin::pin!(h2);
        assert_eq!(h2.max_concurrent_send_streams(), usize::MAX);
        h2.as_mut().await.unwrap();
        // The most-recently advertised value should be used
        assert_eq!(h2.max_concurrent_send_streams(), 2024);
    };

    join(srv, h2).await;
}

// If the server has not sent a go_away message before dropping the connection
// make sure the UnexpectedEof error is propogated.
#[tokio::test]
async fn server_drop_connection_unexpectedly_return_unexpected_eof_err() {
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
        srv.close_without_notify();
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let request = Request::builder()
                .uri("https://http2.akamai.com/")
                .body(())
                .unwrap();
            let _res = client
                .send_request(request, true)
                .unwrap()
                .0
                .await
                .expect("request");
        });
        let err = h2.await.expect_err("should receive UnexpectedEof");
        assert_eq!(
            err.get_io().expect("should be UnexpectedEof").kind(),
            io::ErrorKind::UnexpectedEof,
        );
    };
    join(srv, h2).await;
}

#[tokio::test]
async fn server_drop_connection_after_go_away() {
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
        srv.send_frame(frames::go_away(1)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        srv.close_without_notify();
    };

    let h2 = async move {
        let (mut client, h2) = client::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let request = Request::builder()
                .uri("https://http2.akamai.com/")
                .body(())
                .unwrap();
            let _res = client
                .send_request(request, true)
                .unwrap()
                .0
                .await
                .expect("request");
        });
        let _ = h2.await.unwrap();
    };
    join(srv, h2).await;
}

#[tokio::test]
async fn reset_before_headers_reaches_peer_without_headers() {
    // Repro: body future errors immediately and hyper/h2 converts that into a
    // RST_STREAM before the queued HEADERS are ever written, so the peer sees
    // a reset for an idle stream and treats it as a PROTOCOL_ERROR.
    h2_support::trace_init!();

    let (io, srv) = mock::new();

    // Server task: perform handshake then observe the first frame.
    let srv = async move {
        let mut srv = srv;
        let settings = srv.assert_client_handshake().await;
        assert_default_settings!(settings);

        let frame = tokio::time::timeout(Duration::from_secs(1), srv.next())
            .await
            .expect("timed out waiting for first frame")
            .expect("unexpected EOF")
            .expect("frame error");

        match frame {
            frame::Frame::Headers(h) if h.stream_id() == StreamId::from(1) => {
                assert!(h.is_end_stream() == false);
            }
            frame::Frame::Reset(rst) if rst.stream_id() == StreamId::from(1) => {
                panic!(
                    "BUG: client sent RST_STREAM before any HEADERS on stream 1; reason={:?}",
                    rst.reason()
                );
            }
            other => panic!("unexpected first frame: {:?}", other),
        }
    };

    // Client task: queue HEADERS, immediately reset, then drive the connection.
    let client = async move {
        let (client, conn) = client::handshake(io).await.unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let mut client = client.ready().await.expect("poll_ready");
        let (_resp_fut, mut send_stream) = client.send_request(req, false).unwrap();

        // Simulate body error (reqwest wraps into io::Error::Other) by resetting
        // immediately after the stream is created.
        send_stream.send_reset(Reason::INTERNAL_ERROR);

        // Now start driving the connection so the queued frames get written.
        let conn_task = tokio::spawn(async move {
            let _ = conn.await;
        });

        // Give the connection a moment to flush frames.
        tokio::time::sleep(Duration::from_millis(10)).await;

        drop(send_stream);
        let _ = conn_task.await;
    };

    join(srv, client).await;
}

const SETTINGS: &[u8] = &[0, 0, 0, 4, 0, 0, 0, 0, 0];
const SETTINGS_ACK: &[u8] = &[0, 0, 0, 4, 1, 0, 0, 0, 0];

trait MockH2 {
    fn handshake(&mut self) -> &mut Self;
}

impl MockH2 for mock_io::Builder {
    fn handshake(&mut self) -> &mut Self {
        self.write(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
            // Settings frame
            .write(SETTINGS)
            .read(SETTINGS)
            .read(SETTINGS_ACK)
    }
}

/// RFC 9113 S5.1: "Receiving any frame other than HEADERS or PRIORITY on a
/// stream in [idle] state MUST be treated as a connection error of type
/// PROTOCOL_ERROR."
#[tokio::test]
async fn frame_on_pending_open_stream_is_conn_error() {
    h2_support::trace_init!();

    for scenario in 0..5u8 {
        let (io, mut srv) = mock::new();

        let srv = async move {
            let settings = srv
                .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1))
                .await;
            assert_default_settings!(settings);

            // 3. Receive stream 1 HEADERS.
            srv.recv_frame(
                frames::headers(1)
                    .request("POST", "https://example.com/")
                    .eos(),
            )
            .await;

            idle_ms(50).await;

            // 4. Send a frame targeting stream 3, whose HEADERS haven't
            //    been transmitted since it's pending. This is illegal.
            match scenario {
                0 => {
                    srv.send_frame(frames::reset(3).reason(h2::Reason::NO_ERROR))
                        .await
                }
                1 => {
                    srv.send_frame(frames::reset(3).reason(h2::Reason::CANCEL))
                        .await
                }
                2 => srv.send_frame(frames::window_update(3, 1024)).await,
                3 => srv.send_frame(frames::headers(3).response(200).eos()).await,
                4 => srv.send_frame(frames::data(3, &b"hello"[..])).await,
                _ => unreachable!(),
            }

            // 5. Client responds with GOAWAY(PROTOCOL_ERROR).
            srv.recv_frame(frames::go_away(0).protocol_error()).await;
        };

        let client = async move {
            let (mut client, mut conn) = client::Builder::new()
                .initial_max_send_streams(1)
                .handshake::<_, Bytes>(io)
                .await
                .unwrap();

            // 1. Stream 1 fills the concurrent slot
            let request = Request::builder()
                .method(Method::POST)
                .uri("https://example.com/")
                .body(())
                .unwrap();
            let (_resp1, _) = client.send_request(request, true).unwrap();
            client = conn.drive(client.ready()).await.unwrap();

            // 2. Stream 3 is queued
            let request = Request::builder()
                .method(Method::POST)
                .uri("https://example.com/")
                .body(())
                .unwrap();
            let (_resp3, _) = client.send_request(request, true).unwrap();

            // 6. Connection error propagates to poll_ready.
            conn.drive(client.ready())
                .await
                .expect_err("connection error");
        };

        join(srv, client).await;
    }
}

/// With peer `MAX_CONCURRENT_STREAMS = 0`, `send_request` must fail immediately
/// (`Rejected`) rather than queueing a never-openable `pending_open` stream.
#[tokio::test]
async fn drop_pending_open_with_max_concurrent_streams_zero() {
    use std::time::Duration;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(0))
            .await;
        assert_default_settings!(settings);

        let frame = tokio::time::timeout(Duration::from_secs(2), srv.next())
            .await
            .expect("connection did not close");
        match frame {
            None => {}
            Some(Ok(frame::Frame::GoAway(_))) => {
                srv.recv_eof().await;
            }
            other => panic!("unexpected frame: {:?}", other),
        }
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { conn.await });

        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max_concurrent_streams=0");

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        client
            .send_request(request, true)
            .expect_err("max=0 must Rejected, not queue pending_open");
        drop(client);

        tokio::time::timeout(Duration::from_secs(2), conn)
            .await
            .expect("client connection hung")
            .expect("join conn task")
            .expect("client connection error");
    };

    join(srv, client).await;
}

/// Same as drop_pending_open max=0: send_request is Rejected before any stream
/// exists, so send_reset is not reachable under max=0.
#[tokio::test]
async fn send_reset_pending_open_with_max_concurrent_streams_zero() {
    use std::time::Duration;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(0))
            .await;
        assert_default_settings!(settings);

        let frame = tokio::time::timeout(Duration::from_secs(2), srv.next())
            .await
            .expect("connection did not close");
        match frame {
            None => {}
            Some(Ok(frame::Frame::GoAway(_))) => {
                srv.recv_eof().await;
            }
            other => panic!("unexpected frame: {:?}", other),
        }
    };

    let client = async move {
        let (mut client, conn) = client::handshake(io).await.unwrap();
        let conn = tokio::spawn(async move { conn.await });

        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max_concurrent_streams=0");

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        client
            .send_request(request, false)
            .expect_err("max=0 must Rejected");
        drop(client);

        tokio::time::timeout(Duration::from_secs(2), conn)
            .await
            .expect("client connection hung")
            .expect("join conn task")
            .expect("client connection error");
    };

    join(srv, client).await;
}

/// If `send_reset` queues open-then-RST while a slot exists, then the peer
/// lowers `MAX_CONCURRENT_STREAMS` to 0 before the stream leaves `pending_open`,
/// the stream must still be freed (discard locally — peer never saw it).
#[tokio::test]
async fn send_reset_pending_open_then_max_concurrent_streams_zero() {
    use std::task::Poll;
    use std::time::Duration;
    use tokio::sync::oneshot;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();
    let (reset_done_tx, reset_done_rx) = oneshot::channel();
    let (max0_tx, max0_rx) = oneshot::channel();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(2))
            .await;
        assert_default_settings!(settings);

        // Client queued HEADERS+RST under max=2 without driving the conn.
        reset_done_rx.await.unwrap();

        srv.send_frame(frames::settings().max_concurrent_streams(0))
            .await;
        max0_tx.send(()).unwrap();

        // Must not hang: SETTINGS_ACK, GOAWAY, and/or open-then-RST are all ok.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match srv.next().await {
                    None => return,
                    Some(Ok(frame::Frame::Settings(s))) if s.is_ack() => continue,
                    Some(Ok(frame::Frame::GoAway(_))) => {
                        srv.recv_eof().await;
                        return;
                    }
                    Some(Ok(frame::Frame::Headers(_))) | Some(Ok(frame::Frame::Reset(_))) => {
                        continue;
                    }
                    other => panic!("unexpected frame: {:?}", other),
                }
            }
        })
        .await
        .expect("connection did not settle after max→0");
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        // Drive until max=2 without opening streams.
        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 2 {
                let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                    Poll::Ready(r) => Poll::Ready(r),
                    Poll::Pending => Poll::Ready(Ok(())),
                })
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max=2");

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // Do not poll conn: stream stays pending_open; can_inc true → keep HEADERS+RST.
        let (resp, mut send_stream) = client.send_request(request, false).unwrap();
        send_stream.send_reset(Reason::CANCEL);
        reset_done_tx.send(()).unwrap();
        max0_rx.await.unwrap();

        // Now drive: apply max=0 then free the never-opened reset stream.
        drop(resp);
        drop(send_stream);
        drop(client);

        tokio::time::timeout(Duration::from_secs(2), &mut conn)
            .await
            .expect("client hung after max→0 with reset pending_open")
            .expect("client connection error");
    };

    join(srv, client).await;
}



/// Stream queued under max>0, then max drops to 0 before open: ResponseFuture
/// must resolve with an error (REFUSED_STREAM), not hang.
#[tokio::test]
async fn pending_open_refused_when_max_drops_to_zero() {
    use std::task::Poll;
    use std::time::Duration;
    use tokio::sync::oneshot;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();
    let (queued_tx, queued_rx) = oneshot::channel();
    let (max0_tx, max0_rx) = oneshot::channel();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(2))
            .await;
        assert_default_settings!(settings);

        queued_rx.await.unwrap();
        srv.send_frame(frames::settings().max_concurrent_streams(0))
            .await;
        max0_tx.send(()).unwrap();

        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match srv.next().await {
                    None => return,
                    Some(Ok(frame::Frame::Settings(s))) if s.is_ack() => continue,
                    Some(Ok(frame::Frame::GoAway(_))) => {
                        srv.recv_eof().await;
                        return;
                    }
                    Some(Ok(frame::Frame::Headers(_))) | Some(Ok(frame::Frame::Reset(_))) => {
                        continue;
                    }
                    other => panic!("unexpected frame: {:?}", other),
                }
            }
        })
        .await
        .expect("server settle timeout");
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 2 {
                let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                    Poll::Ready(r) => Poll::Ready(r),
                    Poll::Pending => Poll::Ready(Ok(())),
                })
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max=2");

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/")
            .body(())
            .unwrap();

        // Do not drive conn: stream stays pending_open.
        let (resp, _send) = client.send_request(request, true).unwrap();
        queued_tx.send(()).unwrap();
        max0_rx.await.unwrap();

        // Drive so max=0 is applied and pending_open is aborted.
        let result = tokio::time::timeout(Duration::from_secs(2), async {
            conn.drive(resp).await
        })
        .await
        .expect("ResponseFuture hung after max→0");

        let err = result.expect_err("expected stream error after max→0");
        assert_eq!(err.reason(), Some(Reason::REFUSED_STREAM));

        drop(client);
        let _ = tokio::time::timeout(Duration::from_millis(200), &mut conn).await;
    };

    join(srv, client).await;
}

/// Cancelled `pending_open` streams buried behind a healthy head must still be
/// aborted (not only the head). Otherwise they leak in the slab until the head
/// can open (e.g. while max concurrent remains saturated by a long-lived stream).
#[tokio::test]
async fn cancel_buried_pending_open_is_aborted() {
    use std::task::Poll;
    use std::time::Duration;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        assert_default_settings!(settings);

        // Stream 1 opens and holds the only concurrency slot.
        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/hold")
                .eos(),
        )
        .await;

        // Stream 5 was cancelled while buried behind healthy stream 3 — no wire
        // frames for it. After hold completes, stream 3 opens.
        srv.send_frame(frames::headers(1).response(200).eos()).await;
        srv.recv_frame(
            frames::headers(3)
                .request("GET", "https://example.com/head")
                .eos(),
        )
        .await;
        srv.send_frame(frames::headers(3).response(200).eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        // Wait until peer max concurrent is applied.
        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 1 {
                let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                    Poll::Ready(r) => Poll::Ready(r),
                    Poll::Pending => Poll::Ready(Ok(())),
                })
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max=1");

        // Hold stream: fill max concurrent = 1.
        let hold = Request::builder()
            .uri("https://example.com/hold")
            .body(())
            .unwrap();
        let (hold_resp, hold_send) = client.send_request(hold, true).unwrap();
        // `ready()` stays Pending until hold leaves pending_open (opens on wire).
        client = conn.drive(client.ready()).await.unwrap();
        assert_eq!(client.num_active_streams(), 1);

        // Healthy head of pending_open (slot full) — keep handles so it stays.
        let mut client_head = client.clone();
        let head_req = Request::builder()
            .uri("https://example.com/head")
            .body(())
            .unwrap();
        let (head_resp, head_send) = client_head.send_request(head_req, true).unwrap();

        // Buried *behind* head (clone clears per-handle pending).
        let mut client_buried = client.clone();
        let buried_req = Request::builder()
            .uri("https://example.com/buried")
            .body(())
            .unwrap();
        let (buried_resp, buried_send) = client_buried.send_request(buried_req, true).unwrap();

        // Before cancel: hold + head + buried are wired.
        assert_eq!(client.num_wired_streams(), 3);

        // Cancel the *buried* stream while healthy head still blocks the queue
        // (hold still occupies the only concurrency slot).
        // Drop the clone too: `send_request` may stash the stream in
        // `SendRequest::pending` when occupancy is full, which would keep a ref.
        drop(buried_resp);
        drop(buried_send);
        drop(client_buried);

        // Force poll_complete so abort_closed_pending_open can scan the queue.
        for _ in 0..16 {
            let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                Poll::Ready(r) => Poll::Ready(r),
                Poll::Pending => Poll::Ready(Ok(())),
            })
            .await;
            tokio::task::yield_now().await;
        }

        // Buried stream must be freed already. Pre-fix only aborted the head of
        // pending_open, so a cancelled stream behind a healthy head stayed wired.
        assert_eq!(
            client.num_wired_streams(),
            2,
            "cancelled buried pending_open must free its store slot \
             (hold + head); leaked buried stream would report 3"
        );

        // Complete hold so healthy head can open.
        drop(hold_send);
        let hold_resp = conn.drive(hold_resp).await.expect("hold response");
        assert_eq!(hold_resp.status(), StatusCode::OK);

        let head_resp = conn.drive(head_resp).await.expect("head response");
        assert_eq!(head_resp.status(), StatusCode::OK);
        drop(head_send);

        drop(client);
        tokio::time::timeout(Duration::from_secs(2), conn)
            .await
            .expect("client conn hung")
            .expect("client conn");
    };

    join(srv, client).await;
}


/// Dropping ResponseFuture + SendStream for a pending_open stream must cancel
/// it even when SendRequest still remembers the stream id for backpressure.
/// Otherwise the request is sent after the user cancelled.
#[tokio::test]
async fn drop_stream_handles_cancels_despite_sendrequest_pending() {
    use std::task::Poll;
    use std::time::Duration;
    use tokio::sync::oneshot;
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();
    let (cancel_done_tx, cancel_done_rx) = oneshot::channel();

    let srv = async move {
        let settings = srv
            .assert_client_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        assert_default_settings!(settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/hold")
                .eos(),
        )
        .await;

        // Wait until client has dropped the pending_open stream handles.
        cancel_done_rx.await.unwrap();

        // Free the concurrency slot. Cancelled stream must not appear on wire.
        srv.send_frame(frames::headers(1).response(200).eos()).await;

        let frame = tokio::time::timeout(Duration::from_millis(400), srv.next()).await;
        match frame {
            Err(_) => {} // timeout: no more frames — good
            Ok(None) => {}
            Ok(Some(Ok(frame::Frame::GoAway(_)))) => {}
            Ok(Some(Ok(frame::Frame::Headers(h)))) => {
                panic!(
                    "cancelled pending_open stream should not send HEADERS, got stream {:?}",
                    h.stream_id()
                );
            }
            Ok(Some(other)) => panic!("unexpected frame: {:?}", other),
        }
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();

        tokio::time::timeout(Duration::from_secs(2), async {
            while client.current_max_send_streams() != 1 {
                let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                    Poll::Ready(r) => Poll::Ready(r),
                    Poll::Pending => Poll::Ready(Ok(())),
                })
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for max=1");

        let hold = Request::builder()
            .uri("https://example.com/hold")
            .body(())
            .unwrap();
        let (hold_resp, hold_send) = client.send_request(hold, true).unwrap();
        client = conn.drive(client.ready()).await.unwrap();

        // Queue second request into pending_open; handle records id for ready().
        let cancel_me = Request::builder()
            .uri("https://example.com/cancel-me")
            .body(())
            .unwrap();
        let (resp, send) = client.send_request(cancel_me, true).unwrap();

        // User cancels by dropping stream handles but keeps SendRequest.
        drop(resp);
        drop(send);

        // Drive so abort_closed_pending_open runs while hold still fills max=1.
        for _ in 0..16 {
            let _ = futures::future::poll_fn(|cx| match Pin::new(&mut conn).poll(cx) {
                Poll::Ready(r) => Poll::Ready(r),
                Poll::Pending => Poll::Ready(Ok(())),
            })
            .await;
            tokio::task::yield_now().await;
        }

        // hold (1) only; cancelled stream must be gone from the store.
        assert_eq!(
            client.num_wired_streams(),
            1,
            "dropping ResponseFuture+SendStream must free pending_open stream \
             even when SendRequest still tracks its id for poll_ready"
        );

        cancel_done_tx.send(()).unwrap();

        drop(hold_send);
        let hold_resp = conn.drive(hold_resp).await.expect("hold response");
        assert_eq!(hold_resp.status(), StatusCode::OK);

        // poll_ready must not hang or panic when the tracked stream was cancelled.
        client = conn.drive(client.ready()).await.unwrap();

        drop(client);
        let _ = tokio::time::timeout(Duration::from_millis(200), &mut conn).await;
    };

    join(srv, client).await;
}
