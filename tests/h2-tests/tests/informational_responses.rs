#![deny(warnings)]

use futures::{future::poll_fn, StreamExt};
use h2_support::prelude::*;
use http::{Response, StatusCode};

#[tokio::test]
async fn send_100_continue() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        // Send a POST request
        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;

        // Expect 100 Continue response first
        client
            .recv_frame(frames::headers(1).response(StatusCode::CONTINUE))
            .await;

        // Send request body after receiving 100 Continue
        client
            .send_frame(frames::data(1, &b"request body"[..]).eos())
            .await;

        // Expect final response
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::POST);

        // Send 100 Continue informational response
        let continue_response = Response::builder()
            .status(StatusCode::CONTINUE)
            .body(())
            .unwrap();
        stream.send_informational(continue_response).unwrap();

        // Send final response
        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn send_103_early_hints() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        // Send a GET request
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;

        // Expect 103 Early Hints response first
        client
            .recv_frame(frames::headers(1).response(StatusCode::EARLY_HINTS).field(
                "link",
                "</style.css>; rel=preload; as=style, </script.js>; rel=preload; as=script",
            ))
            .await;

        // Expect final response
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Send 103 Early Hints informational response
        let early_hints_response = Response::builder()
            .status(StatusCode::EARLY_HINTS)
            .header(
                "link",
                "</style.css>; rel=preload; as=style, </script.js>; rel=preload; as=script",
            )
            .body(())
            .unwrap();
        stream.send_informational(early_hints_response).unwrap();

        // Send final response
        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn send_multiple_informational_responses() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;

        // Expect 100 Continue
        client
            .recv_frame(frames::headers(1).response(StatusCode::CONTINUE))
            .await;

        client
            .send_frame(frames::data(1, &b"request body"[..]).eos())
            .await;

        // Expect 103 Early Hints
        client
            .recv_frame(
                frames::headers(1)
                    .response(StatusCode::EARLY_HINTS)
                    .field("link", "</style.css>; rel=preload; as=style"),
            )
            .await;

        // Expect final response
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::POST);

        // Send 100 Continue
        let continue_response = Response::builder()
            .status(StatusCode::CONTINUE)
            .body(())
            .unwrap();
        stream.send_informational(continue_response).unwrap();

        // Send 103 Early Hints
        let early_hints_response = Response::builder()
            .status(StatusCode::EARLY_HINTS)
            .header("link", "</style.css>; rel=preload; as=style")
            .body(())
            .unwrap();
        stream.send_informational(early_hints_response).unwrap();

        // Send final response
        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn invalid_informational_status_returns_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;

        // Should only receive the final response since invalid informational response errors out
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Try to send invalid informational response (200 is not 1xx)
        // This should return an error
        let invalid_response = Response::builder().status(StatusCode::OK).body(()).unwrap();
        let result = stream.send_informational(invalid_response);

        // Expect error for invalid status code
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("invalid informational status code"));

        // Send actual final response after error
        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn client_poll_informational_responses_none() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let (sync_sender, sync_receiver) = tokio::sync::oneshot::channel::<()>();

    let srv = async move {
        let recv_settings = srv.assert_client_handshake().await;
        assert_default_settings!(recv_settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;

        // Send final response directly
        srv.send_frame(frames::headers(1).response(StatusCode::OK))
            .await;

        // The server may not close the stream immediately.
        // Let's simulate this by waiting from client.
        // Continue after the client received the response headers
        tokio::time::timeout(Duration::from_secs(4), sync_receiver)
            .await
            .expect("Client blocked on informational headers")
            .unwrap();
        srv.send_frame(frames::data(1, b"request body").eos()).await;
    };

    let client = async move {
        let (client, connection) = client::handshake(io).await.expect("handshake");

        let request = Request::builder()
            .method("GET")
            .uri("https://example.com/")
            .body(())
            .unwrap();

        let (mut response_future, _) = client
            .ready()
            .await
            .unwrap()
            .send_request(request, true)
            .unwrap();

        tokio::spawn(async move {
            connection.await.expect("connection error");
        });

        // Poll for informational responses
        loop {
            match poll_fn(|cx| response_future.poll_informational(cx)).await {
                Some(Ok(rsp)) => panic!("Unexpected informational response {:?}", rsp),
                Some(Err(e)) => panic!("Error polling informational: {:?}", e),
                None => break,
            }
        }
        // Let the server continue sending responses
        sync_sender.send(()).unwrap();

        // Get the final response
        let response = response_future.await.expect("response error");
        assert_eq!(response.status(), StatusCode::OK);
        let (_hdr, mut recv_stream) = response.into_parts();
        let data = recv_stream.data().await.unwrap().unwrap();
        assert_eq!("request body", data);
    };

    join(srv, client).await;
}

/// poll_informational after the final response must return None, not hang
/// when DATA is already queued (body half still open).
#[tokio::test]
async fn poll_informational_after_final_response_is_none() {
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
        // Final headers then body DATA (no EOS on headers).
        srv.send_frame(frames::headers(1).response(StatusCode::OK))
            .await;
        srv.send_frame(frames::data(1, b"body").eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (mut response_future, _) = client.send_request(request, true).unwrap();

        // Take the final response first (skips any 1xx; none here).
        let response = conn
            .drive(&mut response_future)
            .await
            .expect("final response");
        assert_eq!(response.status(), StatusCode::OK);

        // After final headers were consumed, DATA is at the head of pending_recv.
        // Pre-fix: poll_informational Pending forever while recv half open.
        let info = tokio::time::timeout(
            Duration::from_secs(1),
            poll_fn(|cx| response_future.poll_informational(cx)),
        )
        .await
        .expect("poll_informational hung after final response");
        assert!(info.is_none(), "expected None after final, got {:?}", info);

        let mut body = response.into_body();
        let chunk = conn.drive(body.data()).await.expect("body").expect("ok");
        assert_eq!(chunk.as_ref(), b"body");
        drop(client);
        let _ = conn.await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn client_poll_informational_responses() {
    h2_support::trace_init!();
    let (io, mut srv) = mock::new();

    let srv = async move {
        let recv_settings = srv.assert_client_handshake().await;
        assert_default_settings!(recv_settings);

        srv.recv_frame(
            frames::headers(1)
                .request("GET", "https://example.com/")
                .eos(),
        )
        .await;

        // Send 103 Early Hints
        srv.send_frame(
            frames::headers(1)
                .response(StatusCode::EARLY_HINTS)
                .field("link", "</style.css>; rel=preload"),
        )
        .await;

        // Send final response
        srv.send_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let client = async move {
        let (client, connection) = client::handshake(io).await.expect("handshake");

        let request = Request::builder()
            .method("GET")
            .uri("https://example.com/")
            .body(())
            .unwrap();

        let (mut response_future, _) = client
            .ready()
            .await
            .unwrap()
            .send_request(request, true)
            .unwrap();

        let conn_fut = async move {
            connection.await.expect("connection error");
        };

        let response_fut = async move {
            // Poll for informational responses
            loop {
                match poll_fn(|cx| response_future.poll_informational(cx)).await {
                    Some(Ok(info_response)) => {
                        assert_eq!(info_response.status(), StatusCode::EARLY_HINTS);
                        assert_eq!(
                            info_response.headers().get("link").unwrap(),
                            "</style.css>; rel=preload"
                        );
                        break;
                    }
                    Some(Err(e)) => panic!("Error polling informational: {:?}", e),
                    None => break,
                }
            }

            // Get the final response
            let response = response_future.await.expect("response error");
            assert_eq!(response.status(), StatusCode::OK);
        };

        join(conn_fut, response_fut).await;
    };

    join(srv, client).await;
}

#[tokio::test]
async fn informational_responses_with_body_streaming() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;

        // Expect 100 Continue
        client
            .recv_frame(frames::headers(1).response(StatusCode::CONTINUE))
            .await;

        client.send_frame(frames::data(1, &b"chunk1"[..])).await;

        // Expect 103 Early Hints while still receiving body
        client
            .recv_frame(
                frames::headers(1)
                    .response(StatusCode::EARLY_HINTS)
                    .field("link", "</resource.js>; rel=preload"),
            )
            .await;

        client
            .send_frame(frames::data(1, &b"chunk2"[..]).eos())
            .await;

        // Expect final response with streaming body
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK))
            .await;

        client
            .recv_frame(frames::data(1, &b"response data"[..]).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::POST);

        // Send 100 Continue
        let continue_response = Response::builder()
            .status(StatusCode::CONTINUE)
            .body(())
            .unwrap();
        stream.send_informational(continue_response).unwrap();

        // Send 103 Early Hints while processing
        let early_hints_response = Response::builder()
            .status(StatusCode::EARLY_HINTS)
            .header("link", "</resource.js>; rel=preload")
            .body(())
            .unwrap();
        stream.send_informational(early_hints_response).unwrap();

        // Send final response with body
        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        let mut send_stream = stream.send_response(rsp, false).unwrap();

        send_stream.send_data("response data".into(), true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.1: HTTP/2 does not support 101 Switching Protocols.
#[tokio::test]
async fn switching_protocols_101_is_stream_error() {
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
        srv.send_frame(frames::headers(1).response(101)).await;
        srv.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (resp, _) = client.send_request(request, true).unwrap();
        let err = conn
            .drive(resp)
            .await
            .expect_err("101 Switching Protocols must error");
        assert!(err.is_reset(), "expected stream reset, got {}", err);
        assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
        drop(client);
        let _ = conn.await;
    };

    join(srv, client).await;
}

/// RFC 9113 §8.1: servers must not generate 101 Switching Protocols.
#[tokio::test]
async fn send_informational_rejects_101_switching_protocols() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;

        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        let switching = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(())
            .unwrap();
        let result = stream.send_informational(switching);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("invalid informational status code"));

        let rsp = Response::builder().status(StatusCode::OK).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.1 / Go: informational (1xx) HEADERS must not set END_STREAM.
/// Pre-fix h2 half-closed the receive half and queued InformationalHeaders,
/// leaving the client without a final response.
#[tokio::test]
async fn informational_response_with_end_stream_is_stream_error() {
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
        // 100 Continue with illegal END_STREAM.
        srv.send_frame(frames::headers(1).response(100).eos()).await;
        srv.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (resp, _) = client.send_request(request, true).unwrap();
        let err = conn
            .drive(resp)
            .await
            .expect_err("1xx with END_STREAM must error");
        assert!(err.is_reset(), "expected stream reset, got {}", err);
        assert_eq!(err.reason(), Some(Reason::PROTOCOL_ERROR));
        drop(client);
        let _ = conn.await;
    };

    join(srv, client).await;
}

/// Content-Length on a 1xx response must not bind the final message body.
/// Pre-fix applied CL from 100 Continue to stream.content_length, so a final
/// 200 with a short body (no CL) failed content-length checks.
#[tokio::test]
async fn informational_content_length_does_not_apply_to_final_body() {
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
        // 100 Continue with a misleading Content-Length.
        srv.send_frame(
            frames::headers(1)
                .response(100)
                .field("content-length", 100),
        )
        .await;
        // Final response: no Content-Length, 5-byte body with EOS.
        srv.send_frame(frames::headers(1).response(200)).await;
        srv.send_frame(frames::data(1, &b"hello"[..]).eos()).await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (resp, _) = client.send_request(request, true).unwrap();
        let resp = conn.drive(resp).await.expect("final response");
        assert_eq!(resp.status(), StatusCode::OK);
        let mut body = resp.into_body();
        let chunk = conn
            .drive(body.data())
            .await
            .expect("body chunk")
            .expect("body ok");
        assert_eq!(chunk.as_ref(), b"hello");
        assert!(conn.drive(body.data()).await.is_none());
        drop(client);
        let _ = conn.await;
    };

    join(srv, client).await;
}

/// Cap informational (1xx) HEADERS per stream (Go max1xxResponses = 5).
/// A sixth 1xx is a stream ENHANCE_YOUR_CALM so peers cannot grow pending_recv
/// without bound when the client never drains poll_informational.
#[tokio::test]
async fn too_many_informational_responses_is_stream_error() {
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
        // Five 1xx are allowed.
        for _ in 0..5 {
            srv.send_frame(frames::headers(1).response(100)).await;
        }
        // Sixth is refused.
        srv.send_frame(frames::headers(1).response(100)).await;
        srv.recv_frame(frames::reset(1).reason(Reason::ENHANCE_YOUR_CALM))
            .await;
    };

    let client = async move {
        let (mut client, mut conn) = client::handshake(io).await.unwrap();
        let request = Request::builder()
            .uri("https://example.com/")
            .body(())
            .unwrap();
        let (resp, _) = client.send_request(request, true).unwrap();
        let err = conn
            .drive(resp)
            .await
            .expect_err("sixth 1xx must error the stream");
        assert!(err.is_reset(), "expected stream reset, got {}", err);
        assert_eq!(err.reason(), Some(Reason::ENHANCE_YOUR_CALM));
        drop(client);
        let _ = conn.await;
    };

    join(srv, client).await;
}

/// RFC 9110 §8.6: server MUST NOT send Content-Length on 1xx.
#[tokio::test]
async fn send_informational_rejects_content_length() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .request("POST", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(StatusCode::CONTINUE))
            .await;
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let bad = Response::builder()
            .status(StatusCode::CONTINUE)
            .header("content-length", "0")
            .body(())
            .unwrap();
        let err = stream
            .send_informational(bad)
            .expect_err("1xx with Content-Length must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        stream
            .send_informational(
                Response::builder()
                    .status(StatusCode::CONTINUE)
                    .body(())
                    .unwrap(),
            )
            .expect("1xx without Content-Length ok");
        stream
            .send_response(
                Response::builder().status(StatusCode::OK).body(()).unwrap(),
                true,
            )
            .unwrap();
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Docs: send_informational errors after the final response was sent.
/// Pre-fix still queued 1xx HEADERS on a closed/half-closed send half.
#[tokio::test]
async fn send_informational_after_final_response_is_user_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;

        // Only the final 200; no 1xx after it.
        client
            .recv_frame(frames::headers(1).response(StatusCode::OK).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        stream
            .send_response(
                Response::builder().status(StatusCode::OK).body(()).unwrap(),
                true,
            )
            .expect("final response");

        let cont = Response::builder()
            .status(StatusCode::CONTINUE)
            .body(())
            .unwrap();
        let err = stream
            .send_informational(cont)
            .expect_err("1xx after final response must fail");
        assert!(
            err.to_string().contains("user error")
                || err.to_string().contains("unexpected")
                || err.to_string().contains("already"),
            "got {}",
            err
        );

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}
