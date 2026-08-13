use super::{
    store, Buffer, BufferStatus, Codec, Config, Counts, Frame, Prioritize, Prioritized, Store,
    Stream, StreamId, StreamIdOverflow, WindowSize,
};
use crate::codec::UserError;
use crate::frame::{self, Reason};
use crate::proto::{self, Error, Initiator};

use bytes::Buf;
use tokio::io::AsyncWrite;

use std::cmp::Ordering;
use std::io;
use std::task::{Context, Poll, Waker};

/// Manages state transitions related to outbound frames.
#[derive(Debug)]
pub(super) struct Send {
    /// Stream identifier to use for next initialized stream.
    next_stream_id: Result<StreamId, StreamIdOverflow>,

    /// Any streams with a higher ID are ignored.
    ///
    /// This starts as MAX, but is lowered when a GOAWAY is received.
    ///
    /// > After sending a GOAWAY frame, the sender can discard frames for
    /// > streams initiated by the receiver with identifiers higher than
    /// > the identified last stream.
    max_stream_id: StreamId,

    /// Initial window size of locally initiated streams
    init_window_sz: WindowSize,

    /// Prioritization layer
    prioritize: Prioritize,

    is_push_enabled: bool,

    /// If extended connect protocol is enabled.
    is_extended_connect_protocol_enabled: bool,
}

/// A value to detect which public API has called `poll_reset`.
#[derive(Debug)]
pub(crate) enum PollReset {
    AwaitingHeaders,
    Streaming,
}

impl Send {
    /// Create a new `Send`
    pub fn new(config: &Config) -> Self {
        Send {
            init_window_sz: config.remote_init_window_sz,
            max_stream_id: StreamId::MAX,
            next_stream_id: Ok(config.local_next_stream_id),
            prioritize: Prioritize::new(config),
            is_push_enabled: true,
            is_extended_connect_protocol_enabled: false,
        }
    }

    /// Returns the initial send window size
    pub fn init_window_sz(&self) -> WindowSize {
        self.init_window_sz
    }

    pub fn open(&mut self) -> Result<StreamId, UserError> {
        let stream_id = self.ensure_next_stream_id()?;
        self.next_stream_id = stream_id.next_id();
        Ok(stream_id)
    }

    pub fn reserve_local(&mut self) -> Result<StreamId, UserError> {
        let stream_id = self.ensure_next_stream_id()?;
        self.next_stream_id = stream_id.next_id();
        Ok(stream_id)
    }

    pub(crate) fn is_push_enabled(&self) -> bool {
        self.is_push_enabled
    }

    pub(crate) fn check_headers(fields: &http::HeaderMap) -> Result<(), UserError> {
        // 8.1.2.2. Connection-Specific Header Fields
        if fields.contains_key(http::header::CONNECTION)
            || fields.contains_key(http::header::TRANSFER_ENCODING)
            || fields.contains_key(http::header::UPGRADE)
            || fields.contains_key("keep-alive")
            || fields.contains_key("proxy-connection")
        {
            tracing::debug!("illegal connection-specific headers found");
            return Err(UserError::MalformedHeaders);
        } else {
            // RFC 9110: transfer-coding is case-insensitive; only "trailers" is
            // allowed on HTTP/2 (RFC 9113 §8.2.2). nghttp2 matches case-insensitively.
            for te in fields.get_all(http::header::TE) {
                if !te.as_bytes().eq_ignore_ascii_case(b"trailers") {
                    tracing::debug!("illegal connection-specific headers found");
                    return Err(UserError::MalformedHeaders);
                }
            }
        }

        // RFC 9113 §8.2.1: field values MUST NOT have leading/trailing SP/HTAB.
        for value in fields.values() {
            if frame::header_value_has_leading_trailing_ws(value.as_bytes()) {
                tracing::debug!("header value has leading or trailing whitespace");
                return Err(UserError::MalformedHeaders);
            }
        }

        // RFC 9110 §7.2: more than one Host field is invalid (nghttp2 rejects).
        // Outbound Host is usually stripped when promoted to :authority; still
        // reject multiples if the application left them in the map.
        if fields.get_all(http::header::HOST).iter().count() > 1 {
            tracing::debug!("multiple Host header fields");
            return Err(UserError::MalformedHeaders);
        }
        Ok(())
    }

    pub fn send_push_promise<B>(
        &mut self,
        frame: frame::PushPromise,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        task: &mut Option<Waker>,
    ) -> Result<(), UserError> {
        if !self.is_push_enabled {
            return Err(UserError::PeerDisabledServerPush);
        }

        tracing::trace!(
            "send_push_promise; frame={:?}; init_window={:?}",
            frame,
            self.init_window_sz
        );

        Self::check_headers(frame.fields())?;

        // Queue the frame for sending
        self.prioritize
            .queue_frame(frame.into(), buffer, stream, task);

        Ok(())
    }

    pub fn send_headers<B>(
        &mut self,
        frame: frame::Headers,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), UserError> {
        tracing::trace!(
            "send_headers; frame={:?}; init_window={:?}",
            frame,
            self.init_window_sz
        );

        Self::check_headers(frame.fields())?;

        let end_stream = frame.is_end_stream();

        // Update the state
        stream.state.send_open(end_stream)?;

        let mut pending_open = false;
        if counts.peer().is_local_init(frame.stream_id()) && !stream.is_pending_push {
            self.prioritize.queue_open(stream, counts);
            pending_open = true;
        }

        // Queue the frame for sending
        //
        // This call expects that, since new streams are in the open queue, new
        // streams won't be pushed on pending_send.
        self.prioritize
            .queue_frame(frame.into(), buffer, stream, task);

        // Need to notify the connection when pushing onto pending_open since
        // queue_frame only notifies for pending_send.
        if pending_open {
            if let Some(task) = task.take() {
                task.wake();
            }
        }

        Ok(())
    }

    /// Send interim informational headers (1xx responses) without changing stream state.
    /// This allows multiple interim informational responses to be sent before the final response.
    pub fn send_interim_informational_headers<B>(
        &mut self,
        frame: frame::Headers,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        _counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), UserError> {
        tracing::trace!(
            "send_interim_informational_headers; frame={:?}; stream_id={:?}",
            frame,
            frame.stream_id()
        );

        // Final response (or reset/close) already sent: 1xx must precede it
        // (RFC 9110). Docs promise an error; previously frames were still queued.
        if !stream.state.is_send_informational_allowed() {
            return Err(UserError::UnexpectedFrameType);
        }

        // Validate headers
        Self::check_headers(frame.fields())?;

        debug_assert!(frame.is_informational(),
            "Frame must be informational (1xx status code) at this point. Validation should happen at the public API boundary.");
        debug_assert!(!frame.is_end_stream(),
            "Informational frames must not have end_stream flag set. Validation should happen at the internal send informational header streams.");

        // Queue the frame for sending WITHOUT changing stream state
        // This is the key difference from send_headers - we don't call stream.state.send_open()
        self.prioritize
            .queue_frame(frame.into(), buffer, stream, task);

        Ok(())
    }

    /// Send an explicit RST_STREAM frame
    pub fn send_reset<B>(
        &mut self,
        reason: Reason,
        initiator: Initiator,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        let is_reset = stream.state.is_reset();
        let is_closed = stream.state.is_closed();
        let is_empty = stream.pending_send.is_empty();
        let stream_id = stream.id;

        tracing::trace!(
            "send_reset(..., reason={:?}, initiator={:?}, stream={:?}, ..., \
             is_reset={:?}; is_closed={:?}; pending_send.is_empty={:?}; \
             state={:?} \
             ",
            reason,
            initiator,
            stream_id,
            is_reset,
            is_closed,
            is_empty,
            stream.state
        );

        if is_reset {
            // Don't double reset
            tracing::trace!(
                " -> not sending RST_STREAM ({:?} is already reset)",
                stream_id
            );
            return;
        }

        // Transition the state to reset no matter what.
        stream.set_reset(reason, initiator);

        // If closed AND the send queue is flushed, then the stream cannot be
        // reset explicitly, either. Implicit resets can still be queued.
        if is_closed && is_empty {
            tracing::trace!(
                " -> not sending explicit RST_STREAM ({:?} was closed \
                 and send queue was flushed)",
                stream_id
            );
            return;
        }

        // If the stream hasn't been opened yet (its initial HEADERS are still
        // sitting in `pending_open`/`pending_send`), clearing the queue would
        // drop those HEADERS and let a RST_STREAM become the first frame on an
        // idle stream. HTTP/2 forbids that: §5.1 allows only HEADERS/PRIORITY
        // on idle streams and §6.4 says RST_STREAM on idle is a PROTOCOL_ERROR.
        //
        // When a concurrency slot is available, keep HEADERS and queue RST so
        // the stream opens then resets. When no slot can open (e.g. peer
        // MAX_CONCURRENT_STREAMS=0), discard locally — the peer never saw the
        // stream and waiting forever would leak it in pending_open.
        if stream.is_pending_open {
            if !counts.can_inc_num_send_streams() {
                tracing::trace!(
                    "send_reset -- pending_open with no concurrency slot; discard locally"
                );
                self.prioritize.clear_queue(buffer, stream, counts);
                self.prioritize.reclaim_all_capacity(stream, counts, task);
                // Wake so buffer_pending can abort this stream out of pending_open.
                if let Some(task) = task.take() {
                    task.wake();
                }
                return;
            }
            // Keep HEADERS; queue RST below.
        } else {
            // Drop any buffered DATA/HEADERS and only send the reset.
            //
            // Note that we don't call `self.recv_err` because we want to enqueue
            // the reset frame before transitioning the stream inside
            // `reclaim_all_capacity`.
            self.prioritize.clear_queue(buffer, stream, counts);
        }

        let frame = frame::Reset::new(stream.id, reason);

        tracing::trace!("send_reset -- queueing; frame={:?}", frame);
        self.prioritize
            .queue_frame(frame.into(), buffer, stream, task);
        self.prioritize.reclaim_all_capacity(stream, counts, task);
    }

    pub fn schedule_implicit_reset(
        &mut self,
        stream: &mut store::Ptr,
        reason: Reason,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        if stream.state.is_closed() {
            // Stream is already closed, nothing more to do
            return;
        }

        stream.state.set_scheduled_reset(reason);

        self.prioritize.reclaim_reserved_capacity(stream, counts, task);
        // pending_open streams are not send-ready; still wake the connection so
        // `buffer_pending` can abort them (never opened on the wire).
        // pending_push: schedule_send is also a no-op until PUSH_PROMISE is
        // written; wake so the parent can flush PP, then pop_frame schedules
        // the child for RST.
        if stream.is_pending_open || stream.is_pending_push {
            if let Some(task) = task.take() {
                task.wake();
            }
            return;
        }
        self.prioritize.schedule_send(stream, task);
    }

    pub fn send_data<B>(
        &mut self,
        frame: frame::Data<B>,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), UserError>
    where
        B: Buf,
    {
        self.prioritize
            .send_data(frame, buffer, stream, counts, task)
    }

    pub fn send_trailers<B>(
        &mut self,
        frame: frame::Headers,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), UserError> {
        // Trailers are carried in a HEADERS frame and are therefore subject to the
        // same prohibition on connection-specific header fields (RFC 9113 §8.2.2)
        // as any other outbound HEADERS block. `send_headers`, `send_push_promise`
        // and `send_interim_informational_headers` all validate this, and the
        // receive path treats such a header as malformed. Validate here too so a
        // caller cannot make this crate *generate* a message §8.2.2 forbids.
        // Checked before the state transition so a rejected call leaves the stream
        // untouched and still able to send valid trailers.
        Self::check_headers(frame.fields())?;

        // RFC 9113 §8.1: Content-Length MUST NOT be sent in trailers.
        if frame.fields().contains_key(http::header::CONTENT_LENGTH) {
            return Err(UserError::MalformedHeaders);
        }

        // TODO: Should this logic be moved into state.rs?
        if !stream.state.is_send_streaming() {
            return Err(UserError::UnexpectedFrameType);
        }

        stream.state.send_close();

        tracing::trace!("send_trailers -- queuing; frame={:?}", frame);
        self.prioritize
            .queue_frame(frame.into(), buffer, stream, task);

        // Release any excess capacity
        self.prioritize.reserve_capacity(0, stream, counts, task);

        Ok(())
    }

    pub fn buffer_pending<T, B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        counts: &mut Counts,
        dst: &mut Codec<T, Prioritized<B>>,
    ) -> io::Result<BufferStatus>
    where
        T: AsyncWrite + Unpin,
        B: Buf,
    {
        self.prioritize.buffer_pending(buffer, store, counts, dst)
    }

    pub fn reclaim_written_frame<T, B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        dst: &mut Codec<T, Prioritized<B>>,
    ) -> bool
    where
        B: Buf,
    {
        self.prioritize.reclaim_written_frame(buffer, store, dst)
    }

    /// Request capacity to send data
    pub fn reserve_capacity(
        &mut self,
        capacity: WindowSize,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        self.prioritize
            .reserve_capacity(capacity, stream, counts, task)
    }

    pub fn poll_capacity(
        &mut self,
        cx: &Context,
        stream: &mut store::Ptr,
    ) -> Poll<Option<Result<WindowSize, UserError>>> {
        if !stream.state.is_send_streaming() {
            return Poll::Ready(None);
        }

        let capacity = self.capacity(stream);

        // Never return Ready(Ok(0)) (#898). Usable capacity is
        // min(available, max_send_buffer_size) - buffered. When that is 0,
        // wait for assignment or buffer drain (send_capacity_inc / notify).
        //
        // When capacity > 0, always Ready — including when assigned is still
        // below requested. Callers must be able to send the usable slice of a
        // large reservation without waiting for full assignment (window or
        // max_send_buffer_size may hold available below requested).
        if capacity == 0 {
            stream.send_capacity_inc = false;
            stream.wait_send(cx);
            return Poll::Pending;
        }

        stream.send_capacity_inc = false;
        Poll::Ready(Some(Ok(capacity)))
    }

    /// Current available stream send capacity
    pub fn capacity(&self, stream: &mut store::Ptr) -> WindowSize {
        stream.capacity(self.prioritize.max_buffer_size())
    }

    pub fn poll_reset(
        &self,
        cx: &Context,
        stream: &mut Stream,
        mode: PollReset,
    ) -> Poll<Result<Reason, crate::Error>> {
        match stream.state.ensure_reason(mode)? {
            Some(reason) => Poll::Ready(Ok(reason)),
            None => {
                stream.wait_send(cx);
                Poll::Pending
            }
        }
    }

    pub fn recv_connection_window_update(
        &mut self,
        frame: frame::WindowUpdate,
        store: &mut Store,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), Reason> {
        self.prioritize.recv_connection_window_update(
            frame.size_increment(),
            store,
            counts,
            task,
        )
    }

    pub fn recv_stream_window_update<B>(
        &mut self,
        sz: WindowSize,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), Reason> {
        if let Err(e) = self.prioritize.recv_stream_window_update(sz, stream, task) {
            tracing::debug!("recv_stream_window_update !!; err={:?}", e);

            self.send_reset(
                Reason::FLOW_CONTROL_ERROR,
                Initiator::Library,
                buffer,
                stream,
                counts,
                task,
            );

            return Err(e);
        }

        Ok(())
    }

    pub(super) fn recv_go_away(&mut self, last_stream_id: StreamId) -> Result<(), Error> {
        if last_stream_id > self.max_stream_id {
            // The remote endpoint sent a `GOAWAY` frame indicating a stream
            // that we never sent, or that we have already terminated on account
            // of previous `GOAWAY` frame. In either case, that is illegal.
            // (When sending multiple `GOAWAY`s, "Endpoints MUST NOT increase
            // the value they send in the last stream identifier, since the
            // peers might already have retried unprocessed requests on another
            // connection.")
            proto_err!(conn:
                "recv_go_away: last_stream_id ({:?}) > max_stream_id ({:?})",
                last_stream_id, self.max_stream_id,
            );
            return Err(Error::library_go_away(Reason::PROTOCOL_ERROR));
        }

        self.max_stream_id = last_stream_id;
        Ok(())
    }

    pub fn handle_error<B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
    ) {
        // Clear all pending outbound frames
        self.prioritize.clear_queue(buffer, stream, counts);
        self.prioritize
            .reclaim_all_capacity(stream, counts, &mut None);
    }

    pub fn apply_remote_settings<B>(
        &mut self,
        settings: &frame::Settings,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), Error> {
        if let Some(val) = settings.is_extended_connect_protocol_enabled() {
            self.is_extended_connect_protocol_enabled = val;
        }

        // Applies an update to the remote endpoint's initial window size.
        //
        // Per RFC 7540 §6.9.2:
        //
        // In addition to changing the flow-control window for streams that are
        // not yet active, a SETTINGS frame can alter the initial flow-control
        // window size for streams with active flow-control windows (that is,
        // streams in the "open" or "half-closed (remote)" state). When the
        // value of SETTINGS_INITIAL_WINDOW_SIZE changes, a receiver MUST adjust
        // the size of all stream flow-control windows that it maintains by the
        // difference between the new value and the old value.
        //
        // A change to `SETTINGS_INITIAL_WINDOW_SIZE` can cause the available
        // space in a flow-control window to become negative. A sender MUST
        // track the negative flow-control window and MUST NOT send new
        // flow-controlled frames until it receives WINDOW_UPDATE frames that
        // cause the flow-control window to become positive.
        if let Some(val) = settings.initial_window_size() {
            let old_val = self.init_window_sz;
            self.init_window_sz = val;

            match val.cmp(&old_val) {
                Ordering::Less => {
                    // We must decrease the (remote) window on every open stream.
                    let dec = old_val - val;
                    tracing::trace!("decrementing all windows; dec={}", dec);

                    let mut total_reclaimed = 0;
                    store.try_for_each(|mut stream| {
                        let stream = &mut *stream;

                        if stream.state.is_send_closed() && stream.buffered_send_data == 0 {
                            tracing::trace!(
                                "skipping send-closed stream; id={:?}; flow={:?}",
                                stream.id,
                                stream.send_flow
                            );

                            return Ok(());
                        }

                        tracing::trace!(
                            "decrementing stream window; id={:?}; decr={}; flow={:?}",
                            stream.id,
                            dec,
                            stream.send_flow
                        );

                        // TODO: this decrement can underflow based on received frames!
                        stream
                            .send_flow
                            .dec_send_window(dec)
                            .map_err(proto::Error::library_go_away)?;

                        // It's possible that decreasing the window causes
                        // `window_size` (the stream-specific window) to fall below
                        // `available` (the portion of the connection-level window
                        // that we have allocated to the stream).
                        // In this case, we should take that excess allocation away
                        // and reassign it to other streams.
                        let window_size = stream.send_flow.window_size();
                        let available = stream.send_flow.available().as_size();
                        let reclaimed = if available > window_size {
                            // Drop down to `window_size`.
                            let reclaim = available - window_size;
                            stream
                                .send_flow
                                .claim_capacity(reclaim)
                                .map_err(proto::Error::library_go_away)?;
                            total_reclaimed += reclaim;
                            reclaim
                        } else {
                            0
                        };

                        tracing::trace!(
                            "decremented stream window; id={:?}; decr={}; reclaimed={}; flow={:?}",
                            stream.id,
                            dec,
                            reclaimed,
                            stream.send_flow
                        );

                        // Wake capacity waiters so they observe the reduced
                        // assignment (or zero). Without this, `poll_capacity`
                        // can stay parked until an unrelated increase.
                        if reclaimed > 0 {
                            stream.notify_capacity();
                        }

                        Ok::<_, proto::Error>(())
                    })?;

                    self.prioritize
                        .assign_connection_capacity(total_reclaimed, store, counts, task);
                    self.prioritize
                        .debug_assert_send_capacity_conservation(store);
                }
                Ordering::Greater => {
                    let inc = val - old_val;

                    store.try_for_each(|mut stream| {
                        self.recv_stream_window_update(inc, buffer, &mut stream, counts, task)
                            .map_err(Error::library_go_away)
                    })?;
                    self.prioritize
                        .debug_assert_send_capacity_conservation(store);
                }
                Ordering::Equal => (),
            }
        }

        if let Some(val) = settings.is_push_enabled() {
            // RFC 9113 §6.5.2: "A server MUST NOT send a SETTINGS frame that
            // has [ENABLE_PUSH] set to a value of 1." Clients treat that as a
            // connection error PROTOCOL_ERROR.
            if val && !counts.peer().is_server() {
                proto_err!(conn: "server sent SETTINGS_ENABLE_PUSH = 1");
                return Err(Error::library_go_away(Reason::PROTOCOL_ERROR));
            }
            self.is_push_enabled = val;
        }

        Ok(())
    }

    pub fn clear_queues(&mut self, store: &mut Store, counts: &mut Counts) {
        self.prioritize.clear_pending_capacity(store, counts);
        self.prioritize.clear_pending_send(store, counts);
        self.prioritize.clear_pending_open(store, counts);
    }

    pub fn ensure_not_idle(&self, id: StreamId) -> Result<(), Reason> {
        if let Ok(next) = self.next_stream_id {
            if id >= next {
                return Err(Reason::PROTOCOL_ERROR);
            }
        }
        // if next_stream_id is overflowed, that's ok.

        Ok(())
    }

    pub fn ensure_next_stream_id(&self) -> Result<StreamId, UserError> {
        self.next_stream_id
            .map_err(|_| UserError::OverflowedStreamId)
    }

    pub fn may_have_created_stream(&self, id: StreamId) -> bool {
        if let Ok(next_id) = self.next_stream_id {
            // Peer::is_local_init should have been called beforehand
            debug_assert_eq!(id.is_server_initiated(), next_id.is_server_initiated(),);
            id < next_id
        } else {
            true
        }
    }

    pub(super) fn maybe_reset_next_stream_id(&mut self, id: StreamId) {
        if let Ok(next_id) = self.next_stream_id {
            // Peer::is_local_init should have been called beforehand
            debug_assert_eq!(id.is_server_initiated(), next_id.is_server_initiated());
            if id >= next_id {
                self.next_stream_id = id.next_id();
            }
        }
    }

    pub(crate) fn is_extended_connect_protocol_enabled(&self) -> bool {
        self.is_extended_connect_protocol_enabled
    }
}
