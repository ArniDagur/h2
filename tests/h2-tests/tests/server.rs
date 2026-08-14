#![deny(warnings)]

use futures::StreamExt;
use h2_support::prelude::*;
use std::task::Poll;
use tokio::io::AsyncWriteExt;

const SETTINGS: &[u8] = &[0, 0, 0, 4, 0, 0, 0, 0, 0];
const SETTINGS_ACK: &[u8] = &[0, 0, 0, 4, 1, 0, 0, 0, 0];

#[tokio::test]
async fn read_preface_in_multiple_frames() {
    h2_support::trace_init!();

    let mock = mock_io::Builder::new()
        .read(b"PRI * HTTP/2.0")
        .read(b"\r\n\r\nSM\r\n\r\n")
        .write(SETTINGS)
        .read(SETTINGS)
        .write(SETTINGS_ACK)
        .read(SETTINGS_ACK)
        .build();

    let mut h2 = server::handshake(mock).await.unwrap();

    assert!(h2.next().await.is_none());
}

#[tokio::test]
async fn server_builder_set_max_concurrent_streams() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let mut settings = frame::Settings::default();
    settings.set_max_concurrent_streams(Some(1));

    let client = async move {
        let recv_settings = client.assert_server_handshake().await;
        assert_frame_eq(recv_settings, settings);
        client
            .send_frame(frames::headers(1).request("GET", "https://example.com/"))
            .await;
        client
            .send_frame(frames::headers(3).request("GET", "https://example.com/"))
            .await;
        client
            .send_frame(frames::data(1, &b"hello"[..]).eos())
            .await;
        client.recv_frame(frames::reset(3).refused()).await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let mut builder = server::Builder::new();
    builder.max_concurrent_streams(1);

    let h2 = async move {
        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, h2).await;
}

#[tokio::test]
async fn server_builder_header_table_size() {
    h2_support::trace_init!();

    for size in [0, 10000] {
        let (io, mut client) = mock::new();

        let mut expected = frame::Settings::default();
        expected.set_header_table_size(Some(size));

        let client = async move {
            let recv_settings = client.assert_server_handshake().await;
            assert_frame_eq(recv_settings, expected);
            client
                .send_frame(
                    frames::headers(1)
                        .request("GET", "https://example.com/")
                        .eos(),
                )
                .await;
            client
                .recv_frame(frames::headers(1).response(200).eos())
                .await;
        };

        let mut builder = server::Builder::new();
        builder.header_table_size(size);

        let h2 = async move {
            let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");
            let (req, mut stream) = srv.next().await.unwrap().unwrap();
            assert_eq!(req.method(), &http::Method::GET);
            stream
                .send_response(
                    http::Response::builder().status(200).body(()).unwrap(),
                    true,
                )
                .unwrap();
            assert!(srv.next().await.is_none());
        };

        join(client, h2).await;
    }
}

/// Client may emit a dynamic table size update on the first request HEADERS
/// before ACKing the server's HEADER_TABLE_SIZE increase.
#[tokio::test]
async fn server_header_table_size_increase_applied_before_settings_ack() {
    use std::time::Duration;

    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client.write_preface().await;
        client.send_frame(frames::settings()).await;
        let frame = client.next().await.unwrap().unwrap();
        match frame {
            frame::Frame::Settings(s) => {
                assert_eq!(s.header_table_size(), Some(10000));
            }
            other => panic!("expected server SETTINGS, got {:?}", other),
        }
        // Do not ACK the server's HEADER_TABLE_SIZE SETTINGS.
        // Server will still ACK *our* default SETTINGS.
        client.recv_frame(frames::settings_ack()).await;
        // Size update 10000 then GET https://http2.akamai.com/
        client
            .send_bytes(&[
                0, 0, 0x13, 1, 5, 0, 0, 0, 1, 0x3F, 0xF1, 0x4D, 0x82, 0x87, 0x41, 0x8B, 0x9D, 0x29,
                0xAC, 0x4B, 0x8F, 0xA8, 0xE9, 0x19, 0x97, 0x21, 0xE9, 0x84,
            ])
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let h2 = async move {
        let mut srv = server::Builder::new()
            .header_table_size(10000)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");
        let (req, mut stream) = tokio::time::timeout(Duration::from_secs(2), srv.next())
            .await
            .expect("timed out waiting for request")
            .expect("connection closed")
            .expect("HEADERS with table-size update before SETTINGS_ACK must not GOAWAY");
        assert_eq!(req.method(), &http::Method::GET);
        stream
            .send_response(
                http::Response::builder().status(200).body(()).unwrap(),
                true,
            )
            .unwrap();
        assert!(srv.next().await.is_none());
    };

    join(client, h2).await;
}

#[tokio::test]
async fn serve_request() {
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
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn serve_connect() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("CONNECT", "localhost").eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::CONNECT);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9110 §9.3.6: server MUST NOT send Content-Length in a 2xx CONNECT response.
#[tokio::test]
async fn send_connect_response_rejects_content_length() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("CONNECT", "localhost").eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::CONNECT);

        let bad = http::Response::builder()
            .status(200)
            .header("content-length", "0")
            .body(())
            .unwrap();
        let err = stream
            .send_response(bad, true)
            .expect_err("2xx CONNECT with Content-Length must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        stream
            .send_response(
                http::Response::builder().status(200).body(()).unwrap(),
                true,
            )
            .expect("2xx CONNECT without Content-Length ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn push_request() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
            )
            .await;
        client
            .recv_frame(frames::headers(2).response(200).eos())
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://http2.akamai.com/style2.css"),
            )
            .await;
        client
            .recv_frame(frames::headers(4).response(200).eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Promise stream 2
        let mut pushed_s2 = {
            let req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style.css")
                .body(())
                .unwrap();
            stream.push_request(req).unwrap()
        };

        // Promise stream 4 and push response headers
        {
            let req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style2.css")
                .body(())
                .unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream
                .push_request(req)
                .unwrap()
                .send_response(rsp, true)
                .unwrap();
        }

        // Push response to stream 2
        {
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            pushed_s2.send_response(rsp, true).unwrap();
        }

        // Send response for stream 1
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn push_request_disabled() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().disable_push())
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // attempt to push - expect failure
        let req = http::Request::builder()
            .method("GET")
            .uri("https://http2.akamai.com/style.css")
            .body(())
            .unwrap();
        stream
            .push_request(req)
            .expect_err("push_request should error");

        // send normal response
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn push_request_against_concurrency() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://http2.akamai.com/style2.css"),
            )
            .await;
        client.recv_frame(frames::data(2, &b""[..]).eos()).await;
        client
            .recv_frame(frames::headers(4).response(200).eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Promise stream 2 and start response (concurrency limit reached)
        let mut s2_tx = {
            let req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style.css")
                .body(())
                .unwrap();
            let mut pushed_stream = stream.push_request(req).unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            pushed_stream.send_response(rsp, false).unwrap()
        };

        // Promise stream 4 and push response
        {
            let pushed_req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style2.css")
                .body(())
                .unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream
                .push_request(pushed_req)
                .unwrap()
                .send_response(rsp, true)
                .unwrap();
        }

        // Send and finish response for stream 1
        {
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream.send_response(rsp, true).unwrap();
        }

        // Finish response for stream 2 (at which point stream 4 will be sent)
        s2_tx.send_data(vec![0; 0].into(), true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn push_request_with_data() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(1).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client.recv_frame(frames::data(1, &b""[..]).eos()).await;
        client.recv_frame(frames::data(2, &b"\x00"[..]).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Start response to stream 1
        let mut s1_tx = {
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream.send_response(rsp, false).unwrap()
        };

        // Promise stream 2, push response headers and send data
        {
            let pushed_req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style.css")
                .body(())
                .unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut push_tx = stream
                .push_request(pushed_req)
                .unwrap()
                .send_response(rsp, false)
                .unwrap();
            // Make sure nothing can queue our pushed stream before we have the PushPromise sent
            push_tx.send_data(vec![0; 1].into(), true).unwrap();
            push_tx.reserve_capacity(1);
        }

        // End response for stream 1
        s1_tx.send_data(vec![0; 0].into(), true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn push_request_between_data() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(1).response(200)).await;
        client.recv_frame(frames::data(1, &b""[..])).await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://http2.akamai.com/style.css"),
            )
            .await;
        client
            .recv_frame(frames::headers(2).response(200).eos())
            .await;
        client.recv_frame(frames::data(1, &b""[..]).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        // Push response to stream 1 and send some data
        let mut s1_tx = {
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut tx = stream.send_response(rsp, false).unwrap();
            tx.send_data(vec![0; 0].into(), false).unwrap();
            tx
        };

        // Promise stream 2 and push response headers
        {
            let pushed_req = http::Request::builder()
                .method("GET")
                .uri("https://http2.akamai.com/style.css")
                .body(())
                .unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream
                .push_request(pushed_req)
                .unwrap()
                .send_response(rsp, true)
                .unwrap();
        }

        // End response for stream 1
        s1_tx.send_data(vec![0; 0].into(), true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Resetting the parent before PUSH_PROMISE is flushed must discard the never-
/// sent promised stream (no PP and no RST on the wire). Pre-fix, clear_queue
/// dropped PP frames without freeing the child, leaving a pending_push orphan.
#[tokio::test]
async fn parent_reset_discards_unsent_push_promise_child() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Parent RST only — never-sent PP must not appear, nor a spurious RST(2).
        client.recv_frame(frames::reset(1).cancel()).await;
        let frame = tokio::time::timeout(std::time::Duration::from_millis(200), client.next()).await;
        match frame {
            Err(_) | Ok(None) => {}
            Ok(Some(Ok(frame::Frame::GoAway(_)))) => {}
            Ok(Some(Ok(frame::Frame::PushPromise(pp)))) => {
                panic!("unsent PUSH_PROMISE should be discarded, got {:?}", pp);
            }
            Ok(Some(Ok(frame::Frame::Reset(r)))) => {
                panic!("no RST for never-sent promised stream, got {:?}", r);
            }
            Ok(Some(other)) => panic!("unexpected: {:?}", other),
        }
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        // Queue PP on parent, keep child handle so orphan would stay wired.
        let push = stream.push_request(pushed_req).unwrap();
        // Reset parent before any flush: clears queue including unsent PP.
        stream.send_reset(Reason::CANCEL);
        drop(push);

        // Connection should drain without hanging on orphaned pending_push.
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// After PUSH_PROMISE is on the wire, parent `send_reset` must RST reserved
/// children that never got `send_response` (RFC 9113 §8.4.1). Pre-fix only
/// the parent was reset; the client push future hung on stream 2.
#[tokio::test]
async fn parent_reset_after_push_promise_resets_reserved_child() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();
    let (pp_tx, pp_rx) = tokio::sync::oneshot::channel::<()>();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/style.css"),
            )
            .await;
        let _ = pp_tx.send(());
        client.recv_frame(frames::reset(1).cancel()).await;
        let frame = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(2).cancel()),
        )
        .await
        .expect("reserved push child was not RST after parent cancel");
        let _ = frame;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        // Hold the child so F18 drop-RST does not hide a missing parent-cancel RST.
        let push = stream.push_request(pushed_req).unwrap();

        // Flush PP (client signals) before resetting the parent.
        tokio::select! {
            res = pp_rx => res.unwrap(),
            _ = srv.next() => panic!("unexpected accept while flushing PUSH_PROMISE"),
        }
        stream.send_reset(Reason::CANCEL);
        drop(push);

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Client RST of the parent must also RST reserved advertised push children.
#[tokio::test]
async fn parent_recv_reset_resets_reserved_push_child() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/style.css"),
            )
            .await;
        client.send_frame(frames::reset(1).cancel()).await;
        let frame = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(2).cancel()),
        )
        .await
        .expect("reserved push child was not RST after parent recv RST");
        let _ = frame;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        let push = stream.push_request(pushed_req).unwrap();

        assert!(srv.next().await.is_none());
        drop(push);
        drop(stream);
    };

    join(client, srv).await;
}

/// `try_assign` will give connection send capacity to a `pending_push` child.
/// If the parent is reset before PP is flushed, `clear_queue` discarded the
/// child without reclaiming that capacity (F90) — later streams could not send.
#[tokio::test]
async fn parent_reset_reclaims_unsent_push_child_capacity() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/other")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).cancel()).await;
        client
            .recv_frame(frames::headers(3).response(200))
            .await;
        let data = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::data(3, &b"ok"[..]).eos()),
        )
        .await
        .expect("stream 3 DATA hung: push-child capacity was not reclaimed");
        let _ = data;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        let mut push = stream.push_request(pushed_req).unwrap();
        let mut send = push.send_response(Response::new(()), false).unwrap();
        send.reserve_capacity(65_535);
        send.send_data(Bytes::from(vec![0; 65_535]), true)
            .expect("child send_data");
        drop(send);
        stream.send_reset(Reason::CANCEL);

        let (_req3, mut stream3) = srv.next().await.unwrap().unwrap();
        let mut send3 = stream3.send_response(Response::new(()), false).unwrap();
        send3.send_data(Bytes::from_static(b"ok"), true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// `try_assign` used to give connection send capacity to a `pending_push`
/// child. If MAX_CONCURRENT_STREAMS is already full, popping PP then
/// `queue_open`s that child *with* the window — I1 (pending_open must not
/// hold capacity) and every open stream starves (F91).
#[tokio::test]
async fn pending_push_queued_open_does_not_hoard_send_capacity() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        // After PP(4) the child is pending_open. Stream 2 DATA must still
        // flush — pre-fix the pending_open child held the connection window
        // (I1 panic / hang). HEADERS(1) is not flow-controlled and would
        // arrive *before* DATA(2) on the broken path.
        let data = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::data(2, &b"ok"[..]).eos()),
        )
        .await
        .expect("stream 2 DATA hung: pending_open push child hoarded send capacity");
        let _ = data;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        // Occupy the single server-initiated send slot.
        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let mut send2 = push1
            .send_response(Response::new(()), false)
            .unwrap();

        // Second push: large body. Pre-fix, try_assign hoards the connection
        // window on this still-pending_push child; PP pop then queue_open
        // leaves it there.
        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let mut send4 = push2
            .send_response(Response::new(()), false)
            .unwrap();
        send4.reserve_capacity(65_535);
        send4
            .send_data(Bytes::from(vec![0; 65_535]), true)
            .expect("child 4 send_data");
        drop(send4);

        stream
            .send_response(Response::new(()), true)
            .unwrap();

        send2
            .send_data(Bytes::from_static(b"ok"), true)
            .unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// After PP is on the wire, a push child waiting for a send slot is
/// `pending_open` locally but *reserved* at the peer. RFC §5.1 allows
/// WINDOW_UPDATE there. Treating that id as idle GOAWAY'd the connection (F92).
#[tokio::test]
async fn window_update_on_pending_open_push_is_not_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        // Stream 4 is reserved (remote) for us, pending_open on the server.
        client.send_frame(frames::window_update(4, 1000)).await;
        client.send_frame(frames::ping([0x92; 8])).await;
        let pong = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::ping([0x92; 8]).pong()),
        )
        .await
        .expect("connection died: WU on reserved pending_open push was GOAWAY");
        let _ = pong;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send4 = push2.send_response(Response::new(()), false).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RST_STREAM is also legal on reserved (remote). Same pending_open-as-idle
/// check GOAWAY'd a client that refused a queued push (F92).
#[tokio::test]
async fn reset_on_pending_open_push_is_not_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        client.send_frame(frames::reset(4).cancel()).await;
        client.send_frame(frames::ping([0x93; 8])).await;
        let pong = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::ping([0x93; 8]).pong()),
        )
        .await
        .expect("connection died: RST on reserved pending_open push was GOAWAY");
        let _ = pong;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send4 = push2.send_response(Response::new(()), false).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// DATA on a reserved pending_open push is not idle (F79 residual).
/// F92 already exempted WU/RST; DATA still GOAWAY'd the connection.
#[tokio::test]
async fn data_on_pending_open_push_is_stream_closed_not_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        client.send_frame(frames::data(4, "nope")).await;
        client
            .recv_frame(frames::reset(4).stream_closed())
            .await;
        client.send_frame(frames::ping([0x94; 8])).await;
        let pong = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::ping([0x94; 8]).pong()),
        )
        .await
        .expect("connection died: DATA on reserved pending_open push was GOAWAY");
        let _ = pong;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send4 = push2.send_response(Response::new(()), false).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// After PP is on the wire the child may sit in `pending_open` waiting for a
/// send slot. Dropping that handle used to abort locally (idle-stream rule)
/// and never RST — the peer kept a reserved stream forever (F93).
#[tokio::test]
async fn drop_pending_open_push_sends_reset() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let (flushed_tx, flushed_rx) = tokio::sync::oneshot::channel::<()>();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        // PP(4) is on the wire; child is pending_open on the server.
        let _ = flushed_tx.send(());
        let rst = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(4).cancel()),
        )
        .await
        .expect("RST(4) never sent: reserved pending_open push aborted locally");
        let _ = rst;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let send4 = push2.send_response(Response::new(()), false).unwrap();

        let drive = async {
            assert!(srv.next().await.is_none());
        };
        let cancel = async {
            flushed_rx.await.expect("pp flushed");
            drop(send4);
        };
        join(drive, cancel).await;
    };

    join(client, srv).await;
}

/// `send_response` then drop before PP flushes leaves HEADERS queued on a
/// `pending_push` child. PP pop used to `pending_send.push` without clearing
/// those HEADERS or taking a send slot — the cancelled push opened on the
/// wire and could exceed MAX_CONCURRENT_STREAMS (F94).
#[tokio::test]
async fn drop_push_after_response_before_pp_flush_sends_reset_not_headers() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        let rst = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(4).cancel()),
        )
        .await
        .expect("expected RST(4), not HEADERS: cancelled push was opened");
        let _ = rst;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let send4 = push2.send_response(Response::new(()), false).unwrap();
        drop(send4);

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Explicit `send_reset` on a `pending_push` child `set_reset`s and queues RST,
/// but `schedule_send` is a no-op until PP pops. That path used to `queue_open`
/// the already-reset child; RST then waited for a concurrency slot the reserved
/// stream does not need (F95).
#[tokio::test]
async fn send_reset_pending_push_does_not_wait_for_send_slot() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(1))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/a.css"),
            )
            .await;
        client.recv_frame(frames::headers(2).response(200)).await;
        client
            .recv_frame(
                frames::push_promise(1, 4).request("GET", "https://example.com/b.css"),
            )
            .await;
        let rst = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(4).cancel()),
        )
        .await
        .expect("RST(4) waited for a send slot after explicit send_reset");
        let _ = rst;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut push1 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _send2 = push1.send_response(Response::new(()), false).unwrap();

        let mut push2 = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/b.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let mut send4 = push2.send_response(Response::new(()), false).unwrap();
        send4.send_reset(Reason::CANCEL);

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.4: a client that disabled push MUST treat a later PUSH_PROMISE
/// as connection PROTOCOL_ERROR. poll2 applies SETTINGS then poll_complete
/// writes; a PP queued before ENABLE_PUSH=0 would go out after the flag
/// flipped (F96).
#[tokio::test]
async fn queued_push_promise_not_sent_after_enable_push_zero() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let (queued_tx, queued_rx) = tokio::sync::oneshot::channel::<()>();
    let (drive_tx, drive_rx) = tokio::sync::oneshot::channel::<()>();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        queued_rx.await.expect("pp queued");
        client
            .send_frame(frames::settings().disable_push())
            .await;
        let _ = drive_tx.send(());
        client.recv_frame(frames::settings_ack()).await;
        client.send_frame(frames::ping([0x96; 8])).await;
        let pong = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::ping([0x96; 8]).pong()),
        )
        .await
        .expect("connection died or PUSH_PROMISE sent after ENABLE_PUSH=0");
        let _ = pong;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let _push = stream
            .push_request(
                http::Request::builder()
                    .method("GET")
                    .uri("https://example.com/a.css")
                    .body(())
                    .unwrap(),
            )
            .unwrap();
        let _ = queued_tx.send(());
        // Do not poll_complete until ENABLE_PUSH=0 is in the pipe, or PP
        // is written before SETTINGS is applied.
        drive_rx.await.expect("drive");

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Dropping a promised push stream before `send_response` must still RST after
/// PUSH_PROMISE is on the wire. While `is_pending_push`, `schedule_send` is a
/// no-op, so cancel had to be deferred until PUSH_PROMISE is flushed.
#[tokio::test]
async fn drop_pushed_stream_before_response_sends_reset() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // PP must be sent while parent is still open / half-closed remote.
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/style.css"),
            )
            .await;
        // Without the fix, no RST is ever sent for the cancelled reserved stream.
        client.recv_frame(frames::reset(2).cancel()).await;
        client.recv_frame(frames::headers(1).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        // Create reserved stream then drop without responding (parent still open).
        let push = stream.push_request(pushed_req).unwrap();
        drop(push);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Connection-specific headers / disabled push must not burn a stream id
/// (F25 residual: those checks ran only in send_push_promise after reserve).
#[tokio::test]
async fn push_request_connection_headers_do_not_burn_stream_id() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/ok.css"),
            )
            .await;
        client
            .recv_frame(frames::headers(2).response(200).eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let bad = http::Request::builder()
            .method("GET")
            .uri("https://example.com/x")
            .header("connection", "close")
            .body(())
            .unwrap();
        stream
            .push_request(bad)
            .expect_err("connection header must be UserError");

        let good = http::Request::builder()
            .method("GET")
            .uri("https://example.com/ok.css")
            .body(())
            .unwrap();
        let mut pushed = stream.push_request(good).expect("valid push uses id 2");
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        pushed.send_response(rsp, true).unwrap();

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Invalid `push_request` must not burn a promised stream id (F21 residual).
/// Pre-fix: convert ran after `reserve_local`, so a rejected push advanced
/// the id space and a later good push used stream 4 instead of 2.
#[tokio::test]
async fn push_request_validation_error_does_not_burn_stream_id() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // First successful push must be promised stream 2 (not 4).
        client
            .recv_frame(
                frames::push_promise(1, 2).request("GET", "https://example.com/style.css"),
            )
            .await;
        client
            .recv_frame(frames::headers(2).response(200).eos())
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        // Not safe-and-cacheable → convert_push_message fails.
        let bad = http::Request::builder()
            .method("POST")
            .uri("https://example.com/upload")
            .body(())
            .unwrap();
        stream
            .push_request(bad)
            .expect_err("unsafe method must be UserError");

        // Authority without scheme (HTTP/2 push always requires :scheme).
        let bad = http::Request::builder()
            .method("GET")
            .uri("example.com:8080")
            .body(())
            .unwrap();
        stream
            .push_request(bad)
            .expect_err("missing scheme must be UserError");

        let good = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        let mut pushed = stream.push_request(good).expect("valid push");
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        pushed.send_response(rsp, true).unwrap();

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §6.6: PUSH_PROMISE only on open / half-closed (remote) parents.
#[tokio::test]
async fn push_request_after_response_eos_is_user_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        client
            .assert_server_handshake_with_settings(frames::settings().max_concurrent_streams(100))
            .await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(1).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        let pushed_req = http::Request::builder()
            .method("GET")
            .uri("https://example.com/style.css")
            .body(())
            .unwrap();
        stream
            .push_request(pushed_req)
            .expect_err("PUSH_PROMISE after parent closed must fail");

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[test]
#[ignore]
fn accept_with_pending_connections_after_socket_close() {}

#[tokio::test]
async fn recv_invalid_authority() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let bad_auth = util::byte_str("not:a/good authority");
    let mut bad_headers: frame::Headers = frames::headers(1)
        .request("GET", "https://example.com/")
        .eos()
        .into();
    bad_headers.pseudo_mut().authority = Some(bad_auth);

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client.send_frame(bad_headers).await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn recv_connection_header() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let req = |id, name, val| {
        frames::headers(id)
            .request("GET", "https://example.com/")
            .field(name, val)
            .eos()
    };

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client.send_frame(req(1, "connection", "foo")).await;
        client.send_frame(req(3, "keep-alive", "5")).await;
        client.send_frame(req(5, "proxy-connection", "bar")).await;
        client
            .send_frame(req(7, "transfer-encoding", "chunked"))
            .await;
        client.send_frame(req(9, "upgrade", "HTTP/2")).await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        client.recv_frame(frames::reset(3).protocol_error()).await;
        client.recv_frame(frames::reset(5).protocol_error()).await;
        client.recv_frame(frames::reset(7).protocol_error()).await;
        client.recv_frame(frames::reset(9).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.2.1 / nghttp2: uppercase field names are stream-malformed.
/// Pre-fix `HeaderName::from_lowercase` failed as HPACK → GOAWAY PROTOCOL_ERROR.
#[tokio::test]
async fn recv_uppercase_header_name_is_stream_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    // HEADERS+EOS stream 1: GET https://example.com/ + literal "X-Foo: bar"
    let mut frame = vec![0, 0, 0x1b, 1, 5, 0, 0, 0, 1, 0x82, 0x87, 0x84, 0x41, 0x0b];
    frame.extend_from_slice(b"example.com");
    frame.extend_from_slice(&[0x00, 0x05]);
    frame.extend_from_slice(b"X-Foo");
    frame.extend_from_slice(&[0x03]);
    frame.extend_from_slice(b"bar");
    assert_eq!(frame.len(), 9 + 0x1b);

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client.send_bytes(&frame).await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/ok")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(3).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/ok");
        let _ = stream.send_response(Response::builder().status(200).body(()).unwrap(), true);
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.2.1: empty field names are malformed (stream), not NeedMore.
/// Pre-fix `Header::new` returned NeedMore → codec GOAWAY PROTOCOL_ERROR.
#[tokio::test]
async fn recv_empty_header_name_is_stream_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    // HEADERS+EOS stream 1: GET https://example.com/ + literal empty-name "foo"
    let mut frame = vec![0, 0, 0x16, 1, 5, 0, 0, 0, 1, 0x82, 0x87, 0x84, 0x41, 0x0b];
    frame.extend_from_slice(b"example.com");
    frame.extend_from_slice(&[0x00, 0x00, 0x03]);
    frame.extend_from_slice(b"foo");
    assert_eq!(frame.len(), 9 + 0x16);

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client.send_bytes(&frame).await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/ok")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(3).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/ok");
        let _ = stream.send_response(Response::builder().status(200).body(()).unwrap(), true);
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.2.1: field values MUST NOT have leading or trailing SP/HTAB.
/// nghttp2 rejects these; http::HeaderValue accepts them so h2 must check.
#[tokio::test]
async fn recv_header_value_leading_trailing_ws_is_stream_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);

        let leading = http::HeaderValue::from_bytes(b" value").unwrap();
        let trailing = http::HeaderValue::from_bytes(b"value ").unwrap();
        let tab = http::HeaderValue::from_bytes(b"\tvalue").unwrap();

        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("x-a", leading)
                    .eos(),
            )
            .await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .field("x-b", trailing)
                    .eos(),
            )
            .await;
        client
            .send_frame(
                frames::headers(5)
                    .request("GET", "https://example.com/")
                    .field("x-c", tab)
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        client.recv_frame(frames::reset(3).protocol_error()).await;
        client.recv_frame(frames::reset(5).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Connection-specific headers in a block that spans CONTINUATION.
///
/// The first 16KiB frame typically finishes `connection` and then `NeedMore`
/// on a large following field. Pre-fix dropped the malformed flag (accepted
/// the request) or, if decode completed the first frame, RST'd and treated
/// the rest of the block as unexpected CONTINUATION (GOAWAY).
#[tokio::test]
async fn recv_connection_header_spanning_continuation_is_stream_error() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let pad = "x".repeat(20_000);

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("connection", "close")
                    .field("x-pad", pad)
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/ok")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(3).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/ok");
        let _ = stream.send_response(Response::builder().status(200).body(()).unwrap(), true);
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn sends_reset_no_error_when_req_body_is_dropped() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        // server responded with data before consuming POST-request's body, resulting in `RST_STREAM(NO_ERROR)`.
        client.recv_frame(frames::headers(1).response(200)).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client
            .recv_frame(frames::data(1, vec![0; 16384]).eos())
            .await;
        client
            .recv_frame(frames::reset(1).reason(Reason::NO_ERROR))
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        {
            let (req, mut stream) = srv.next().await.unwrap().unwrap();
            assert_eq!(req.method(), &http::Method::POST);

            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut tx = stream.send_response(rsp, false).unwrap();
            tx.send_data(vec![0; 16384 * 2].into(), true).unwrap();
        }
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn no_error_response_body_delivered_before_rst() {
    // When a server sends a large response body and drops the request
    // body without reading it, NO_ERROR is scheduled. The response DATA
    // must still be delivered.
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        client.recv_frame(frames::headers(1).response(200)).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client.recv_frame(frames::data(1, vec![0; 16383])).await;
        // These window updates allow the full response to be delivered.
        client.send_frame(frames::window_update(0, 65535)).await;
        client.send_frame(frames::window_update(1, 65535)).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client.recv_frame(frames::data(1, vec![0; 16384])).await;
        client.recv_frame(frames::data(1, vec![0; 1]).eos()).await;
        client
            .recv_frame(frames::reset(1).reason(Reason::NO_ERROR))
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        {
            let (req, mut stream) = srv.next().await.unwrap().unwrap();
            assert_eq!(req.method(), &http::Method::POST);

            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut tx = stream.send_response(rsp, false).unwrap();
            // Response body larger than the stream window. The first 65535 bytes
            // are sent immediately, and the remaining bytes wait for the client's
            // WINDOW_UPDATE.
            tx.send_data(vec![0; 16384 * 6].into(), true).unwrap();
        }
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// F30 residual of #896: early-response NO_ERROR waits to flush response DATA.
/// If the peer advertised INITIAL_WINDOW_SIZE=0, that DATA can never leave and
/// a NO_ERROR schedule would hang the connection. maybe_cancel must use CANCEL
/// when the stream window is already closed so the body is discarded and RST
/// is emitted promptly.
#[tokio::test]
async fn early_response_zero_window_uses_cancel_not_hang() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client
            .assert_server_handshake_with_settings(frames::settings().initial_window_size(0))
            .await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        client.recv_frame(frames::headers(1).response(200)).await;
        // CANCEL (not NO_ERROR): response body is unsendable under zero window.
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.recv_frame(frames::reset(1).cancel()),
        )
        .await
        .expect("RST_STREAM not received within 2s with zero stream window");
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        {
            let (req, mut stream) = srv.next().await.unwrap().unwrap();
            assert_eq!(req.method(), &http::Method::POST);
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut tx = stream.send_response(rsp, false).unwrap();
            // Buffer body with EOS while peer stream window is 0.
            tx.send_data(vec![0; 10].into(), true).unwrap();
            // Drop handles → must not schedule NO_ERROR (would hang).
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            assert!(srv.next().await.is_none());
        })
        .await
        .expect("server connection hung with early response + zero window");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn abrupt_shutdown() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).request("POST", "https://example.com/"))
            .await;
        client.recv_frame(frames::go_away(1).internal_error()).await;
        client.recv_eof().await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, tx) = srv.next().await.unwrap().expect("server receives request");

        let req_fut = async move {
            let body = util::concat(req.into_body()).await;
            drop(tx);
            let err = body.expect_err("request body should error");
            assert_eq!(
                err.reason(),
                Some(Reason::INTERNAL_ERROR),
                "streams should be also error with user's reason",
            );
        };

        srv.abrupt_shutdown(Reason::INTERNAL_ERROR);

        let srv_fut = async move {
            poll_fn(move |cx| srv.poll_closed(cx))
                .await
                .expect("server");
        };

        join(req_fut, srv_fut).await;
    };

    join(client, srv).await;
}

#[tokio::test]
async fn graceful_shutdown() {
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
        // 2^31 - 1 = 2147483647
        // Note: not using a constant in the library because library devs
        // can be unsmart.
        client.recv_frame(frames::go_away(2147483647)).await;
        client.recv_frame(frames::ping(frame::Ping::SHUTDOWN)).await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
        // Pretend this stream was sent while the GOAWAY was in flight
        client
            .send_frame(frames::headers(3).request("POST", "https://example.com/"))
            .await;
        client
            .send_frame(frames::ping(frame::Ping::SHUTDOWN).pong())
            .await;
        client.recv_frame(frames::go_away(3)).await;
        // streams sent after GOAWAY receive no response
        client
            .send_frame(frames::headers(7).request("GET", "https://example.com/"))
            .await;
        client.send_frame(frames::data(7, "").eos()).await;
        client.send_frame(frames::data(3, "").eos()).await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
        client.recv_eof().await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        srv.graceful_shutdown();

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::POST);
        let body = req.into_parts().1;

        let body = async move {
            let buf = util::concat(body).await.unwrap();
            assert!(buf.is_empty());

            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream.send_response(rsp, true).unwrap();
        };

        let mut srv = Box::pin(async move {
            assert!(srv.next().await.is_none(), "unexpected request");
        });
        srv.drive(body).await;
        srv.await;
    };

    join(client, srv).await;
}

#[tokio::test]
async fn goaway_even_if_client_sent_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(5)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Ping-pong so as to wait until server gets req
        client.ping_pong([0; 8]).await;
        client.send_frame(frames::go_away(0)).await;
        // 2^31 - 1 = 2147483647
        // Note: not using a constant in the library because library devs
        // can be unsmart.
        client.recv_frame(frames::go_away(2147483647)).await;
        client.recv_frame(frames::ping(frame::Ping::SHUTDOWN)).await;
        client
            .recv_frame(frames::headers(5).response(200).eos())
            .await;
        client
            .send_frame(frames::ping(frame::Ping::SHUTDOWN).pong())
            .await;
        client.recv_frame(frames::go_away(5)).await;
        client.recv_eof().await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        srv.graceful_shutdown();

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none(), "unexpected request");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn client_goaway_does_not_kill_remote_initiated_streams() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        // Client sends a request on stream 1
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Receive response headers (no END_STREAM)
        client.recv_frame(frames::headers(1).response(200)).await;
        // Client sends GOAWAY(0)
        client.send_frame(frames::go_away(0)).await;
        // Server should still be able to send the response body
        client
            .recv_frame(frames::data(1, "the response body").eos())
            .await;
        // Server sends its own GOAWAY and closes
        client.recv_frame(frames::go_away(1)).await;
        client.recv_eof().await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), &http::Method::GET);

        // Send response headers without END_STREAM
        let rsp = http::Response::builder().status(200).body(()).unwrap();
        let mut tx = stream.send_response(rsp, false).unwrap();

        // Drive the connection while sending the body.
        // The yields ensure the connection processes the client's GOAWAY
        // before we attempt to send data.
        let send_body = async {
            // First yield: connection flushes headers. Client receives them
            // and sends GOAWAY(0).
            tokio::task::yield_now().await;
            // Second yield: connection reads and processes GOAWAY(0).
            // Before the fix, stream 1 was killed here.
            tokio::task::yield_now().await;
            // Send response body. Before the fix, this failed because
            // stream 1 was incorrectly closed by recv_go_away.
            tx.send_data("the response body".into(), true).unwrap();
        };

        let mut srv = Box::pin(async move {
            assert!(srv.next().await.is_none(), "unexpected request");
        });
        srv.drive(send_body).await;
        srv.await;
    };

    join(client, srv).await;
}

#[tokio::test]
async fn sends_reset_cancel_when_res_body_is_dropped() {
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
        client.recv_frame(frames::headers(1).response(200)).await;
        client.recv_frame(frames::reset(1).cancel()).await;
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::headers(3).response(200)).await;
        // CANCEL means "stream is no longer needed" (RFC 9113 §7). Buffered DATA
        // is discarded and RST_STREAM is sent immediately.
        client.recv_frame(frames::reset(3).cancel()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        {
            let (req, mut stream) = srv.next().await.unwrap().unwrap();

            assert_eq!(req.method(), &http::Method::GET);

            let rsp = http::Response::builder().status(200).body(()).unwrap();
            stream.send_response(rsp, false).unwrap();
            // SendStream dropped
        }
        {
            let (_req, mut stream) = srv.next().await.unwrap().unwrap();
            let rsp = http::Response::builder().status(200).body(()).unwrap();
            let mut tx = stream.send_response(rsp, false).unwrap();
            tx.send_data(vec![0; 10].into(), false).unwrap();
            // no send_data with eos
        }

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn too_big_headers_sends_431() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(64));
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("some-header", "some-value")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(431).eos())
            .await;
        idle_ms(10).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .max_header_list_size(64)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let req = srv.next().await;
        assert!(req.is_none(), "req is {:?}", req);
    };

    join(client, srv).await;
}

#[tokio::test]
async fn too_big_headers_sends_reset_after_431_if_not_eos() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(64));
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("some-header", "some-value"),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(431).eos())
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .max_header_list_size(64)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let req = srv.next().await;
        assert!(req.is_none(), "req is {:?}", req);
    };

    join(client, srv).await;
}

#[tokio::test]
async fn abusive_headers_send_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(64));
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("x-abuse", "a".repeat(200))
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::go_away(0).calm().data("header_list_way_too_large"))
            .await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .max_header_list_size(64)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let err = srv.next().await.unwrap().expect_err("server");
        assert!(err.is_go_away());
        assert!(err.is_library());
        assert_eq!(err.reason(), Some(Reason::ENHANCE_YOUR_CALM));
    };

    join(client, srv).await;
}

#[tokio::test]
async fn too_many_continuation_frames_sends_goaway() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_frame_eq(settings, frames::settings().max_header_list_size(1024 * 32));

        // the mock impl automatically splits into CONTINUATION frames if the
        // headers are too big for one frame. So without a max header list size
        // set, we'll send a bunch of headers that will eventually get nuked.
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("a".repeat(10_000), "b".repeat(10_000))
                    .field("c".repeat(10_000), "d".repeat(10_000))
                    .field("e".repeat(10_000), "f".repeat(10_000))
                    .field("g".repeat(10_000), "h".repeat(10_000))
                    .field("i".repeat(10_000), "j".repeat(10_000))
                    .field("k".repeat(10_000), "l".repeat(10_000))
                    .field("m".repeat(10_000), "n".repeat(10_000))
                    .field("o".repeat(10_000), "p".repeat(10_000))
                    .field("y".repeat(10_000), "z".repeat(10_000)),
            )
            .await;
        client
            .recv_frame(frames::go_away(0).calm().data("too_many_continuations"))
            .await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            // should mean ~3 continuation
            .max_header_list_size(1024 * 32)
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let err = srv.next().await.unwrap().expect_err("server");
        assert!(err.is_go_away());
        assert!(err.is_library());
        assert_eq!(err.reason(), Some(Reason::ENHANCE_YOUR_CALM));
    };

    join(client, srv).await;
}

#[tokio::test]
async fn pending_accept_recv_illegal_content_length_data() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .request("POST", "https://a.b")
                    .field("content-length", "1"),
            )
            .await;
        client
            .send_frame(frames::data(1, &b"hello"[..]).eos())
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
        idle_ms(10).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let _req = srv.next().await.expect("req").expect("is_ok");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn poll_reset() {
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
        idle_ms(10).await;
        client.send_frame(frames::reset(1).cancel()).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");
        let (_req, mut tx) = srv.next().await.expect("server").unwrap();
        let conn = async move {
            let req = srv.next().await;
            assert!(req.is_none(), "no second request");
        };
        join(conn, async move {
            let reason = poll_fn(move |cx| tx.poll_reset(cx))
                .await
                .expect("poll_reset");
            assert_eq!(reason, Reason::CANCEL);
        })
        .await;
    };
    join(client, srv).await;
}

#[tokio::test]
async fn poll_reset_io_error() {
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
        idle_ms(10).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let (_req, mut tx) = srv.next().await.expect("server").unwrap();
        let conn = async move {
            let req = srv.next().await;
            assert!(req.is_none(), "no second request");
        };
        join(conn, async move {
            poll_fn(move |cx| tx.poll_reset(cx))
                .await
                .expect_err("poll_reset should error")
        })
        .await;
    };

    join(client, srv).await;
}

#[tokio::test]
async fn poll_reset_after_send_response_is_user_error() {
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
        client.recv_frame(frames::headers(1).response(200)).await;
        client
            .recv_frame(
                // After the error, our server will drop the handles,
                // meaning we receive a RST_STREAM here.
                frames::reset(1).cancel(),
            )
            .await;
        idle_ms(10).await;
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        let (_req, mut tx) = srv.next().await.expect("server").expect("request");
        let conn = async move {
            let req = srv.next().await;
            assert!(req.is_none(), "no second request");
        };
        tx.send_response(Response::new(()), false)
            .expect("response");
        drop(_req);
        join(
            async {
                poll_fn(move |cx| tx.poll_reset(cx))
                    .await
                    .expect_err("poll_reset should error")
            },
            conn,
        )
        .await;
    };

    join(client, srv).await;
}

#[tokio::test]
async fn server_error_on_unclean_shutdown() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let srv = server::Builder::new().handshake::<_, Bytes>(io);

    client.write_all(b"PRI *").await.expect("write");
    drop(client);

    srv.await.expect_err("should error");
}

#[tokio::test]
async fn server_error_on_status_in_request() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(frames::headers(1).status(StatusCode::OK))
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// Origin-form may omit :authority when Host is present (HTTP/1.1→H2 style).
#[tokio::test]
async fn request_with_host_without_authority_pseudo() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "/just-a-path")
                    .scheme("http")
                    .field("host", "example.com")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/just-a-path");
        assert_eq!(req.uri().authority().unwrap().as_str(), "example.com");

        let rsp = Response::new(());
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9110 §4.3.1: empty host identifier is forbidden. http::Authority
/// accepts `":80"` / `":"` with host "".
#[tokio::test]
async fn reject_request_empty_host_in_authority() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        authority: util::byte_str(":80").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().authority().unwrap().as_str(), "example.com");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 3986 §3.2.2: empty IP-literal `[]` is not a valid host. http::Authority
/// accepts it with host `"[]"` (F66 residual).
#[tokio::test]
async fn reject_request_empty_ipv6_literal_authority() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        authority: util::byte_str("[]").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        // Host-only path with empty IP-literal.
        client
            .send_frame(
                frames::headers(3)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .field("host", "[]:443")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(3).protocol_error()).await;

        // Valid IPv6 unspecified still accepted.
        client
            .send_frame(
                frames::headers(5)
                    .request("GET", "https://[::1]/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(5).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().host().unwrap(), "[::1]");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 9113 §8.3.1 / nghttp2: :path for http(s) must be path-absolute ("/"…)
/// or OPTIONS "*". PathAndQuery accepts query-only "?q=1"; that is not valid
/// as :path. Pre-fix accepted it.
#[tokio::test]
async fn reject_request_path_without_leading_slash() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        authority: util::byte_str("example.com").into(),
                        path: util::byte_str("?q=1").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/?q=1")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/");
        assert_eq!(req.uri().query(), Some("q=1"));
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// nghttp2 / RFC 9113: non-CONNECT requests need :authority or Host.
/// Pre-fix accepted scheme+path only (not routable).
#[tokio::test]
async fn reject_request_without_authority_or_host() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "/just-a-path")
                    .scheme("http")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn serve_when_request_in_response_extensions() {
    use std::sync::Arc;
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
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut rsp = http::Response::new(());
        rsp.extensions_mut().insert(Arc::new(req));
        stream.send_response(rsp, true).unwrap();

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn send_reset_explicitly() {
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
            .recv_frame(frames::reset(1).reason(Reason::ENHANCE_YOUR_CALM))
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        stream.send_reset(Reason::ENHANCE_YOUR_CALM);

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn send_reset_explicitly_does_not_affect_local_limit() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_default_settings!(settings);
        for s in (1..9).step_by(2) {
            client
                .send_frame(
                    frames::headers(s)
                        .request("GET", "https://example.com/")
                        .eos(),
                )
                .await;
            client
                .recv_frame(frames::reset(s).reason(Reason::INTERNAL_ERROR))
                .await;
        }
    };

    let srv = async move {
        let mut srv = server::Builder::new()
            .max_local_error_reset_streams(Some(3))
            .handshake::<_, Bytes>(io)
            .await
            .expect("handshake");

        for _s in (1..9).step_by(2) {
            let (_req, mut stream) = srv.next().await.unwrap().unwrap();
            stream.send_reset(Reason::INTERNAL_ERROR);
        }

        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn extended_connect_protocol_disabled_by_default() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;

        assert_eq!(settings.is_extended_connect_protocol_enabled(), None);

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo::request(
                Method::CONNECT,
                uri::Uri::from_static("http://bread/baguette"),
                Protocol::from_static("the-bread-protocol").into(),
            )))
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn extended_connect_protocol_enabled_during_handshake() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;

        assert_eq!(settings.is_extended_connect_protocol_enabled(), Some(true));

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo::request(
                Method::CONNECT,
                uri::Uri::from_static("http://bread/baguette"),
                Protocol::from_static("the-bread-protocol").into(),
            )))
            .await;

        client.recv_frame(frames::headers(1).response(200)).await;
    };

    let srv = async move {
        let mut builder = server::Builder::new();

        builder.enable_connect_protocol();

        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");

        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(
            req.extensions().get::<crate::ext::Protocol>(),
            Some(&crate::ext::Protocol::from_static("the-bread-protocol"))
        );

        let rsp = Response::new(());
        stream.send_response(rsp, false).unwrap();

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_pseudo_protocol_on_non_connect_request() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;

        assert_eq!(settings.is_extended_connect_protocol_enabled(), Some(true));

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo::request(
                Method::GET,
                uri::Uri::from_static("http://bread/baguette"),
                Some(Protocol::from_static("the-bread-protocol")),
            )))
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut builder = server::Builder::new();

        builder.enable_connect_protocol();

        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");

        assert!(srv.next().await.is_none());

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// Mid-connection `enable_connect_protocol` must accept `:protocol` before
/// SETTINGS_ACK (peer may use ENABLE=1 as soon as it processes SETTINGS).
#[tokio::test]
async fn enable_connect_protocol_before_settings_ack() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_eq!(settings.is_extended_connect_protocol_enabled(), None);

        let frame = client.next().await.unwrap().unwrap();
        match frame {
            frame::Frame::Settings(s) => {
                assert_eq!(s.is_extended_connect_protocol_enabled(), Some(true));
                assert!(!s.is_ack());
            }
            other => panic!("expected ENABLE_CONNECT SETTINGS, got {:?}", other),
        }
        // Deliberately no SETTINGS_ACK.

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo::request(
                Method::CONNECT,
                uri::Uri::from_static("http://bread/baguette"),
                Protocol::from_static("the-bread-protocol").into(),
            )))
            .await;

        client.recv_frame(frames::headers(1).response(200).eos()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        // Initial SETTINGS is WaitingAck until the client's ACK is read.
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if srv.enable_connect_protocol().is_ok() {
                    break;
                }
                poll_fn(|cx| {
                    match srv.poll_closed(cx) {
                        Poll::Ready(r) => r.expect("server closed while enabling"),
                        Poll::Pending => {}
                    }
                    Poll::Ready(())
                })
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("timed out enabling connect protocol");

        let (req, mut stream) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            srv.next(),
        )
        .await
        .expect("timed out waiting for extended CONNECT")
        .expect("connection closed")
        .expect(":protocol before SETTINGS_ACK must not RST");

        assert_eq!(
            req.extensions().get::<crate::ext::Protocol>(),
            Some(&crate::ext::Protocol::from_static("the-bread-protocol"))
        );

        stream
            .send_response(Response::new(()), true)
            .unwrap();

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// nghttp2 rejects empty pseudo-header values; empty :protocol is not a valid
/// ALPN token (RFC 8441). Pre-fix accepted empty Protocol and treated the
/// request as extended CONNECT.
#[tokio::test]
async fn reject_empty_protocol_pseudo() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;
        assert_eq!(settings.is_extended_connect_protocol_enabled(), Some(true));

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo::request(
                Method::CONNECT,
                uri::Uri::from_static("http://example.com/"),
                Some(Protocol::from_static("")),
            )))
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        // Leading/trailing SP/HTAB on :protocol also rejected (RFC 9113 §8.2.1).
        client
            .send_frame(frames::headers(3).pseudo(frame::Pseudo::request(
                Method::CONNECT,
                uri::Uri::from_static("http://example.com/"),
                Some(Protocol::from_static(" websocket")),
            )))
            .await;
        client.recv_frame(frames::reset(3).protocol_error()).await;
    };

    let srv = async move {
        let mut builder = server::Builder::new();
        builder.enable_connect_protocol();
        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_extended_connect_request_without_scheme() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;

        assert_eq!(settings.is_extended_connect_protocol_enabled(), Some(true));

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo {
                method: Method::CONNECT.into(),
                path: util::byte_str("/").into(),
                protocol: Protocol::from("the-bread-protocol").into(),
                ..Default::default()
            }))
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut builder = server::Builder::new();

        builder.enable_connect_protocol();

        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");

        assert!(srv.next().await.is_none());

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_extended_connect_request_without_path() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let settings = client.assert_server_handshake().await;

        assert_eq!(settings.is_extended_connect_protocol_enabled(), Some(true));

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo {
                method: Method::CONNECT.into(),
                scheme: util::byte_str("https").into(),
                protocol: Protocol::from("the-bread-protocol").into(),
                ..Default::default()
            }))
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut builder = server::Builder::new();

        builder.enable_connect_protocol();

        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");

        assert!(srv.next().await.is_none());

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}


/// RFC 9113 §8.3.1: non-CONNECT requests MUST include :path.
/// With authority, http::Uri::from_parts fails ("path missing") so RST is sent
/// via the builder error path. With scheme only (no authority), h2 drops the
/// scheme and previously accepted a request with empty path.

/// RFC 9113 §8.3.1: server SHOULD treat request as malformed if Host
/// differs from :authority.

/// Matching Host and :authority is allowed (RFC does not forbid duplicates
/// when they identify the same entity).
#[tokio::test]
async fn matching_host_with_authority_is_accepted() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("host", "example.com")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().authority().unwrap().as_str(), "example.com");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}


/// RFC 9113 §8.3.1: :authority MUST NOT include the deprecated userinfo
/// subcomponent (user:pass@host).


/// Final send_response must not send 1xx (use send_informational).
/// Pre-fix converted 100 + EOS into on-wire HEADERS that clients reject (F33).
#[tokio::test]
async fn send_response_rejects_informational_status() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        // Server should not send 1xx via send_response; expect a normal final response after reject.
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();
        let cont = Response::builder().status(100).body(()).unwrap();
        let err = stream
            .send_response(cont, true)
            .expect_err("1xx via send_response must fail");
        assert!(
            err.to_string().contains("user error") || err.to_string().contains("informational"),
            "got {}",
            err
        );
        stream
            .send_response(Response::new(()), true)
            .expect("final 200 ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9110 §8.6: generate-path mismatched multi Content-Length is invalid.
#[tokio::test]
async fn send_response_rejects_mismatched_content_length() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let mut bad = Response::builder().status(200).body(()).unwrap();
        bad.headers_mut().append("content-length", "5".parse().unwrap());
        bad.headers_mut().append("content-length", "6".parse().unwrap());
        let err = stream
            .send_response(bad, false)
            .expect_err("mismatched Content-Length must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        stream
            .send_response(Response::builder().status(200).body(()).unwrap(), true)
            .expect("clean 200 ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9113 §8.1.1: END_STREAM + non-zero Content-Length is malformed
/// (304 may still advertise representation length with empty body).
#[tokio::test]
async fn send_response_rejects_nonzero_content_length_with_end_stream() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let bad = Response::builder()
            .status(200)
            .header("content-length", "10")
            .body(())
            .unwrap();
        let err = stream
            .send_response(bad, true)
            .expect_err("200 + non-zero CL + EOS must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        // 304 MAY carry Content-Length of the selected representation.
        // Not asserted here — only that a clean 200 still works after reject.
        stream
            .send_response(Response::builder().status(200).body(()).unwrap(), true)
            .expect("clean 200 ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9110 §8.6: server MUST NOT send Content-Length on 204.
/// 205 requires empty content — non-zero CL is rejected; 304 MAY include CL.
#[tokio::test]
async fn send_response_rejects_content_length_on_no_content() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(204).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();

        let with_cl = Response::builder()
            .status(204)
            .header("content-length", "5")
            .body(())
            .unwrap();
        let err = stream
            .send_response(with_cl, true)
            .expect_err("204 with Content-Length must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        let nonzero_205 = Response::builder()
            .status(205)
            .header("content-length", "1")
            .body(())
            .unwrap();
        let err = stream
            .send_response(nonzero_205, true)
            .expect_err("205 with non-zero Content-Length must fail");
        assert!(
            err.to_string().contains("malformed") || err.to_string().contains("user error"),
            "got {}",
            err
        );

        stream
            .send_response(Response::builder().status(204).body(()).unwrap(), true)
            .expect("204 without Content-Length ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

/// RFC 9110: 204/205/304 are terminated by the header section.
/// send_response(..., false) would emit HEADERS without END_STREAM (peers RST via F43).
#[tokio::test]
async fn send_response_rejects_no_content_without_end_stream() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(1).response(204).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (_req, mut stream) = srv.next().await.unwrap().unwrap();
        let no_content = Response::builder().status(204).body(()).unwrap();
        let err = stream
            .send_response(no_content, false)
            .expect_err("204 without end_stream must fail");
        assert!(
            err.to_string().contains("user error") || err.to_string().contains("unexpected"),
            "got {}",
            err
        );
        stream
            .send_response(Response::builder().status(204).body(()).unwrap(), true)
            .expect("204 with end_stream ok");
        assert!(srv.next().await.is_none());
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_authority_with_userinfo() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo {
                method: Method::GET.into(),
                scheme: util::byte_str("https").into(),
                authority: util::byte_str("user:pass@example.com").into(),
                path: util::byte_str("/").into(),
                ..Default::default()
            }).eos())
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none(), "userinfo in :authority must not be accepted");
        poll_fn(move |cx| srv.poll_closed(cx)).await.expect("server");
    };

    join(client, srv).await;
}

/// Host-only origin-form: same userinfo ban as :authority (F44). Pre-fix only
/// checked @ on the :authority pseudo, so Host: user@host was accepted.
#[tokio::test]
async fn reject_host_header_with_userinfo() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .field("host", "user:pass@example.com")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none(), "userinfo in Host must not be accepted");
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_host_header_differing_from_authority() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .field("host", "evil.example")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none(), "mismatched Host must not be accepted");
        poll_fn(move |cx| srv.poll_closed(cx)).await.expect("server");
    };

    join(client, srv).await;
}

/// RFC 9110 §7.2 / nghttp2: more than one Host field is invalid.
/// Pre-fix accepted multiples; only the first was compared to :authority.
#[tokio::test]
async fn reject_multiple_host_headers() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        let mut fields = http::HeaderMap::new();
        fields.append(http::header::HOST, "example.com".parse().unwrap());
        fields.append(http::header::HOST, "evil.example".parse().unwrap());
        client
            .send_frame(
                frames::headers(1)
                    .request("GET", "https://example.com/")
                    .fields(fields)
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none(), "multiple Host must not be accepted");
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_request_missing_path_pseudo() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        // method + scheme, no authority, no :path — the subtle case
        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo {
                method: Method::GET.into(),
                scheme: util::byte_str("https").into(),
                ..Default::default()
            }).eos())
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;

        // Also with authority (builder-path coverage)
        client
            .send_frame(frames::headers(3).pseudo(frame::Pseudo {
                method: Method::GET.into(),
                scheme: util::byte_str("https").into(),
                authority: util::byte_str("example.com").into(),
                ..Default::default()
            }).eos())
            .await;

        client.recv_frame(frames::reset(3).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        // Must not deliver a request without :path
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 9110 §7.1: asterisk-form request-target (`*`) is only for OPTIONS.
/// nghttp2 enforces the same for http/https. Pre-fix PathAndQuery accepted `*`.
#[tokio::test]
async fn reject_asterisk_path_for_non_options() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        // GET with :path = * is malformed.
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("https").into(),
                        authority: util::byte_str("example.com").into(),
                        path: util::byte_str("*").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        // OPTIONS * remains valid.
        client
            .send_frame(
                frames::headers(3)
                    .pseudo(frame::Pseudo {
                        method: Method::OPTIONS.into(),
                        scheme: util::byte_str("https").into(),
                        authority: util::byte_str("example.com").into(),
                        path: util::byte_str("*").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.method(), Method::OPTIONS);
        assert_eq!(req.uri().path(), "*");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 3986 §3.1 / nghttp2: scheme must start with ALPHA (not a digit).
/// `http::uri::Scheme` accepts `"1http"`.
#[tokio::test]
async fn reject_request_digit_leading_scheme() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("1http").into(),
                        authority: util::byte_str("example.com").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;

        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 3986 §3.1 / RFC 9113 §8.3.1: `:scheme` must be a non-empty scheme token.
/// `http::uri::Scheme` accepts `""`, so empty was previously treated as present.
#[tokio::test]
async fn reject_request_empty_scheme_pseudo() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::GET.into(),
                        scheme: util::byte_str("").into(),
                        authority: util::byte_str("example.com").into(),
                        path: util::byte_str("/").into(),
                        ..Default::default()
                    })
                    .eos(),
            )
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;

        // Follow-up valid request still works on the connection.
        client
            .send_frame(
                frames::headers(3)
                    .request("GET", "https://example.com/")
                    .eos(),
            )
            .await;
        client
            .recv_frame(frames::headers(3).response(200).eos())
            .await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        // Empty scheme must not be delivered as a request.
        let (req, mut stream) = srv.next().await.unwrap().unwrap();
        assert_eq!(req.uri().path(), "/");
        stream.send_response(Response::new(()), true).unwrap();
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 9110 §9.3.6: traditional CONNECT must not include Content-Length.
#[tokio::test]
async fn reject_connect_with_content_length() {
    h2_support::trace_init!();
    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;
        client
            .send_frame(
                frames::headers(1)
                    .pseudo(frame::Pseudo {
                        method: Method::CONNECT.into(),
                        authority: util::byte_str("tunnel.example.com:443").into(),
                        ..Default::default()
                    })
                    .field("content-length", "0")
                    .eos(),
            )
            .await;
        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(
            srv.next().await.is_none(),
            "CONNECT with Content-Length must not be accepted"
        );
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

/// RFC 9113 §8.5 / §8.3.1: CONNECT requests MUST include :authority.
#[tokio::test]
async fn reject_connect_missing_authority_pseudo() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        client
            .send_frame(frames::headers(1).pseudo(frame::Pseudo {
                method: Method::CONNECT.into(),
                // no authority, scheme, or path (correct for CONNECT except missing authority)
                ..Default::default()
            }).eos())
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let mut srv = server::handshake(io).await.expect("handshake");
        assert!(srv.next().await.is_none());
        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn reject_informational_status_header_in_request() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();

    let client = async move {
        let _ = client.assert_server_handshake().await;

        let status_code = 128;
        assert!(StatusCode::from_u16(status_code)
            .unwrap()
            .is_informational());

        client
            .send_frame(frames::headers(1).response(status_code))
            .await;

        client.recv_frame(frames::reset(1).protocol_error()).await;
    };

    let srv = async move {
        let builder = server::Builder::new();
        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");

        poll_fn(move |cx| srv.poll_closed(cx))
            .await
            .expect("server");
    };

    join(client, srv).await;
}

#[tokio::test]
async fn client_drop_connection_without_close_notify() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();
    let client = async move {
        let _recv_settings = client.assert_server_handshake().await;
        client
            .send_frame(frames::headers(1).request("GET", "https://example.com/"))
            .await;
        client.send_frame(frames::data(1, &b"hello"[..])).await;
        client.recv_frame(frames::headers(1).response(200)).await;

        client.close_without_notify(); // Client closed without notify causing UnexpectedEof
    };

    let mut builder = server::Builder::new();
    builder.max_concurrent_streams(1);

    let h2 = async move {
        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, false).unwrap();

        // Step the conn state forward and hitting the EOF
        // But we have no outstanding request from client to be satisfied, so we should not return
        // an error
        let _ = poll_fn(|cx| srv.poll_closed(cx)).await.unwrap();
    };

    join(client, h2).await;
}

#[tokio::test]
async fn init_window_size_smaller_than_default_should_use_default_before_ack() {
    h2_support::trace_init!();

    let (io, mut client) = mock::new();
    let client = async move {
        // Client can send in some data before ACK;
        // Server needs to make sure the Recv stream has default window size
        // as per https://datatracker.ietf.org/doc/html/rfc9113#name-initial-flow-control-window
        client.write_preface().await;
        client
            .send(frame::Settings::default().into())
            .await
            .unwrap();
        client.next().await.expect("unexpected EOF").unwrap();
        client
            .send_frame(frames::headers(1).request("GET", "https://example.com/"))
            .await;
        client.send_frame(frames::data(1, &b"hello"[..])).await;
        client.send(frame::Settings::ack().into()).await.unwrap();
        client.next().await;
        client
            .recv_frame(frames::headers(1).response(200).eos())
            .await;
    };

    let mut builder = server::Builder::new();
    builder.max_concurrent_streams(1);
    builder.initial_window_size(1);
    let h2 = async move {
        let mut srv = builder.handshake::<_, Bytes>(io).await.expect("handshake");
        let (req, mut stream) = srv.next().await.unwrap().unwrap();

        assert_eq!(req.method(), &http::Method::GET);

        let rsp = http::Response::builder().status(200).body(()).unwrap();
        stream.send_response(rsp, true).unwrap();

        // Drive the state forward
        let _ = poll_fn(|cx| srv.poll_closed(cx)).await.unwrap();
    };

    join(client, h2).await;
}

#[tokio::test]
async fn remote_reset_does_not_panic_connection_driver() {
    h2_support::trace_init!();

    const ADVERSARIAL_WIRE: &[u8] = &[
        // Client connection preface.
        0x50, 0x52, 0x49, 0x20, 0x2a, 0x20, 0x48, 0x54, 0x54, 0x50, 0x2f, 0x32, 0x2e, 0x30, 0x0d,
        0x0a, 0x0d, 0x0a, 0x53, 0x4d, 0x0d, 0x0a, 0x0d, 0x0a,
        // SETTINGS len=0, flags=0, stream=0.
        0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
        // Unknown frame type 0x87, len=5, flags=0xc1, stream=257.
        0x00, 0x00, 0x05, 0x87, 0xc1, 0x00, 0x00, 0x01, 0x01, 0x05, 0x94, 0x05, 0x01, 0x00,
        // Unknown frame type 0xc1, len=0, flags=0x94, stream=1281.
        0x00, 0x00, 0x00, 0xc1, 0x94, 0x00, 0x00, 0x05, 0x01,
        // HEADERS len=4, flags=END_STREAM | END_HEADERS, stream=4353.
        0x00, 0x00, 0x04, 0x01, 0x05, 0x00, 0x00, 0x11, 0x01, 0x83, 0x87, 0x01, 0x00,
        // RST_STREAM len=4, flags=0x05, stream=4353.
        0x00, 0x00, 0x04, 0x03, 0x05, 0x00, 0x00, 0x11, 0x01, 0x83, 0x87, 0x01, 0x00,
        // HEADERS len=5, flags=0xf6, stream=4353.
        0x00, 0x00, 0x05, 0x01, 0xf6, 0x00, 0x00, 0x11, 0x01, 0x01, 0x94, 0x00, 0x3d, 0x01,
        // PUSH_PROMISE len=5, flags=0xf6, stream=4353.
        0x00, 0x00, 0x05, 0x05, 0xf6, 0x00, 0x00, 0x11, 0x01, 0x3d, 0x94, 0x81, 0x00, 0x95,
        // HEADERS len=0, flags=END_STREAM | END_HEADERS, stream=4353.
        0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x11, 0x01,
    ];

    let (mut client_io, server_io) = tokio::io::duplex(256 * 1024);
    let server_task = tokio::spawn(async move {
        let Ok(mut server) = server::handshake(server_io).await else {
            return;
        };

        while let Some(result) = server.next().await {
            let _ = result;
        }
    });

    client_io
        .write_all(ADVERSARIAL_WIRE)
        .await
        .expect("write adversarial wire");
    drop(client_io);

    tokio::time::timeout(std::time::Duration::from_secs(1), server_task)
        .await
        .expect("server task timed out")
        .expect("server task panicked");
}
