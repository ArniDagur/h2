use super::store::Resolve;
use super::*;

use crate::frame::Reason;

use crate::codec::UserError;
use crate::codec::UserError::*;

use bytes::buf::Take;
use std::{
    cmp::{self, Ordering},
    fmt, io, mem,
    task::Waker,
};

/// # Warning
///
/// Queued streams are ordered by stream ID, as we need to ensure that
/// lower-numbered streams are sent headers before higher-numbered ones.
/// This is because "idle" stream IDs – those which have been initiated but
/// have yet to receive frames – will be implicitly closed on receipt of a
/// frame on a higher stream ID. If these queues was not ordered by stream
/// IDs, some mechanism would be necessary to ensure that the lowest-numbered]
/// idle stream is opened first.
#[derive(Debug)]
pub(super) struct Prioritize {
    /// Queue of streams waiting for socket capacity to send a frame.
    pending_send: store::Queue<stream::NextSend>,

    /// Queue of streams waiting for window capacity to produce data.
    pending_capacity: store::Queue<stream::NextSendCapacity>,

    /// Streams waiting for capacity due to max concurrency
    ///
    /// The `SendRequest` handle is `Clone`. This enables initiating requests
    /// from many tasks. However, offering this capability while supporting
    /// backpressure at some level is tricky. If there are many `SendRequest`
    /// handles and a single stream becomes available, which handle gets
    /// assigned that stream? Maybe that handle is no longer ready to send a
    /// request.
    ///
    /// The strategy used is to allow each `SendRequest` handle one buffered
    /// request. A `SendRequest` handle is ready to send a request if it has no
    /// associated buffered requests. This is the same strategy as `mpsc` in the
    /// futures library.
    pending_open: store::Queue<stream::NextOpen>,

    /// Connection level flow control governing sent data
    flow: FlowControl,

    /// Stream ID of the last stream opened.
    last_opened_id: StreamId,

    /// What `DATA` frame is currently being sent in the codec.
    in_flight_data_frame: InFlightData,

    /// The maximum amount of bytes a stream should buffer.
    max_buffer_size: usize,
}

#[derive(Debug, Eq, PartialEq)]
enum InFlightData {
    /// There is no `DATA` frame in flight.
    Nothing,
    /// There is a `DATA` frame in flight belonging to the given stream.
    DataFrame(store::Key),
    /// There was a `DATA` frame, but the stream's queue was since cleared.
    Drop,
}

pub(crate) struct Prioritized<B> {
    // The buffer
    inner: Take<B>,

    end_of_stream: bool,

    // The stream that this is associated with
    stream: store::Key,
}

// ===== impl Prioritize =====

impl Prioritize {
    pub fn new(config: &Config) -> Prioritize {
        let mut flow = FlowControl::new();

        flow.inc_window(config.remote_init_window_sz)
            .expect("invalid initial window size");

        // TODO: proper error handling
        let _res = flow.assign_capacity(config.remote_init_window_sz);
        debug_assert!(_res.is_ok());

        tracing::trace!("Prioritize::new; flow={:?}", flow);

        Prioritize {
            pending_send: store::Queue::new(),
            pending_capacity: store::Queue::new(),
            pending_open: store::Queue::new(),
            flow,
            last_opened_id: StreamId::ZERO,
            in_flight_data_frame: InFlightData::Nothing,
            max_buffer_size: config.local_max_buffer_size,
        }
    }

    pub(crate) fn max_buffer_size(&self) -> usize {
        self.max_buffer_size
    }

    /// Connection-level send capacity conservation:
    /// `sum(stream.send available) + conn.available == conn.window`.
    ///
    /// Assigning capacity to streams moves available between stream and
    /// connection; sending DATA decreases both the stream available and the
    /// connection window by the same amount (via reclaim-then-send on the
    /// connection flow controller).
    #[cfg(debug_assertions)]
    pub(super) fn debug_assert_send_capacity_conservation(&self, store: &Store) {
        let stream_sum = store.sum_send_available_signed();
        let conn_avail = self.flow.available_signed() as i64;
        let conn_window = self.flow.window_size_signed() as i64;
        debug_assert_eq!(
            stream_sum + conn_avail,
            conn_window,
            "send capacity conservation violated: \
             stream_available_sum={stream_sum} conn_available={conn_avail} \
             conn_window={conn_window}"
        );
        if let Some(id) = store.pending_open_holds_send_capacity() {
            panic!(
                "pending_open stream {id:?} holds send capacity (starves open streams)"
            );
        }
    }

    #[cfg(not(debug_assertions))]
    #[inline]
    pub(super) fn debug_assert_send_capacity_conservation(&self, _store: &Store) {}

    /// Queue a frame to be sent to the remote
    pub fn queue_frame<B>(
        &mut self,
        frame: Frame<B>,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        task: &mut Option<Waker>,
    ) {
        let span = tracing::trace_span!("Prioritize::queue_frame", ?stream.id);
        let _e = span.enter();
        // Queue the frame in the buffer
        stream.pending_send.push_back(buffer, frame);
        self.schedule_send(stream, task);
    }

    pub fn schedule_send(&mut self, stream: &mut store::Ptr, task: &mut Option<Waker>) {
        // If the stream is waiting to be opened, nothing more to do.
        if stream.is_send_ready() {
            tracing::trace!(?stream.id, "schedule_send");
            // Queue the stream
            self.pending_send.push(stream);

            // Notify the connection.
            if let Some(task) = task.take() {
                task.wake();
            }
        }
    }

    pub fn queue_open(&mut self, stream: &mut store::Ptr, counts: &mut Counts) {
        if self.pending_open.push(stream) {
            counts.inc_num_pending_open();
        }
    }

    /// Send a data frame
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
        let sz = frame.payload().remaining();

        if sz > MAX_WINDOW_SIZE as usize {
            return Err(UserError::PayloadTooBig);
        }

        let sz = sz as WindowSize;

        if !stream.state.is_send_streaming() {
            if stream.state.is_closed() {
                return Err(InactiveStreamId);
            } else {
                return Err(UnexpectedFrameType);
            }
        }

        // Update the buffered data counter
        stream.buffered_send_data += sz as usize;

        let span =
            tracing::trace_span!("send_data", sz, requested = stream.requested_send_capacity);
        let _e = span.enter();
        tracing::trace!(buffered = stream.buffered_send_data);

        // Implicitly request more send capacity if not enough has been
        // requested yet.
        if (stream.requested_send_capacity as usize) < stream.buffered_send_data {
            // Update the target requested capacity (HTTP/2 max window, not u32::MAX)
            stream.requested_send_capacity =
                cmp::min(stream.buffered_send_data, MAX_WINDOW_SIZE as usize) as WindowSize;

            // `try_assign_capacity` will queue the stream to `pending_capacity` if the capcaity
            // cannot be assigned at the time it is called.
            self.try_assign_capacity(stream, task);
        }

        if frame.is_end_stream() {
            stream.state.send_close();
            self.reserve_capacity(0, stream, counts, task);
            stream.notify_send_if_closed();
        }

        tracing::trace!(
            available = %stream.send_flow.available(),
            buffered = stream.buffered_send_data,
        );

        // The `stream.buffered_send_data == 0` check is here so that, if a zero
        // length data frame is queued to the front (there is no previously
        // queued data), it gets sent out immediately even if there is no
        // available send window.
        //
        // Sending out zero length data frames can be done to signal
        // end-of-stream.
        //
        if stream.send_flow.available() > 0 || stream.buffered_send_data == 0 {
            // The stream currently has capacity to send the data frame, so
            // queue it up and notify the connection task.
            self.queue_frame(frame.into(), buffer, stream, task);
        } else {
            // The stream has no capacity to send the frame now, save it but
            // don't notify the connection task. Once additional capacity
            // becomes available, the frame will be flushed.
            stream.pending_send.push_back(buffer, frame.into());
        }

        Ok(())
    }

    /// Request capacity to send data
    pub fn reserve_capacity(
        &mut self,
        capacity: WindowSize,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        let span = tracing::trace_span!(
            "reserve_capacity",
            ?stream.id,
            requested = capacity,
            effective = (capacity as usize) + stream.buffered_send_data,
            curr = stream.requested_send_capacity
        );
        let _e = span.enter();

        // Actual capacity is `capacity` + the current amount of buffered data.
        // If it were less, then we could never send out the buffered data.
        // Cap at MAX_WINDOW_SIZE (not WindowSize::MAX / u32::MAX): peer windows
        // cannot exceed 2^31-1 (RFC 9113 §6.9.1).
        let capacity =
            ((capacity as usize) + stream.buffered_send_data).min(MAX_WINDOW_SIZE as usize);

        match capacity.cmp(&(stream.requested_send_capacity as usize)) {
            Ordering::Equal => {
                // Nothing to do
            }
            Ordering::Less => {
                // Update the target requested capacity
                stream.requested_send_capacity = capacity as WindowSize;

                // Currently available capacity assigned to the stream
                let available = stream.send_flow.available().as_size();

                // If the stream has more assigned capacity than requested, reclaim
                // some for the connection
                if available as usize > capacity {
                    let diff = available - capacity as WindowSize;

                    // TODO: proper error handling
                    let _res = stream.send_flow.claim_capacity(diff);
                    debug_assert!(_res.is_ok());

                    self.assign_connection_capacity(diff, stream, counts, task);
                }
            }
            Ordering::Greater => {
                // If trying to *add* capacity, but the stream send side is closed,
                // there's nothing to be done.
                if stream.state.is_send_closed() {
                    return;
                }

                // Update the target requested capacity
                stream.requested_send_capacity = capacity as WindowSize;

                // Try to assign additional capacity to the stream. If none is
                // currently available, the stream will be queued to receive some
                // when more becomes available.
                self.try_assign_capacity(stream, task);
            }
        }
    }

    pub fn recv_stream_window_update(
        &mut self,
        inc: WindowSize,
        stream: &mut store::Ptr,
        task: &mut Option<Waker>,
    ) -> Result<(), Reason> {
        let span = tracing::trace_span!(
            "recv_stream_window_update",
            ?stream.id,
            ?stream.state,
            inc,
            flow = ?stream.send_flow
        );
        let _e = span.enter();

        if stream.state.is_send_closed() && stream.buffered_send_data == 0 {
            // We can't send any data, so don't bother doing anything else.
            return Ok(());
        }

        // Update the stream level flow control.
        stream.send_flow.inc_window(inc)?;

        // If the stream is waiting on additional capacity, then this will
        // assign it (if available on the connection) and notify the producer
        self.try_assign_capacity(stream, task);

        Ok(())
    }

    pub fn recv_connection_window_update(
        &mut self,
        inc: WindowSize,
        store: &mut Store,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) -> Result<(), Reason> {
        // Update the connection's window
        self.flow.inc_window(inc)?;

        self.assign_connection_capacity(inc, store, counts, task);
        self.debug_assert_send_capacity_conservation(store);
        Ok(())
    }

    /// Reclaim all capacity assigned to the stream and re-assign it to the
    /// connection
    pub fn reclaim_all_capacity(
        &mut self,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        let available = stream.send_flow.available().as_size();
        if available > 0 {
            // TODO: proper error handling
            let _res = stream.send_flow.claim_capacity(available);
            debug_assert!(_res.is_ok());
            // Re-assign all capacity to the connection
            self.assign_connection_capacity(available, stream, counts, task);
        }
    }

    /// Reclaim just reserved capacity, not buffered capacity, and re-assign
    /// it to the connection
    pub fn reclaim_reserved_capacity(
        &mut self,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        // only reclaim reserved capacity that isn't already buffered
        if stream.send_flow.available().as_size() as usize > stream.buffered_send_data {
            let reserved =
                stream.send_flow.available().as_size() - stream.buffered_send_data as WindowSize;

            // Panic safety: due to how `reserved` is computed it can't be greater
            // than what's available.
            stream
                .send_flow
                .claim_capacity(reserved)
                .expect("window size should be greater than reserved");

            self.assign_connection_capacity(reserved, stream, counts, task);
        }
    }

    pub fn clear_pending_capacity(&mut self, store: &mut Store, counts: &mut Counts) {
        let span = tracing::trace_span!("clear_pending_capacity");
        let _e = span.enter();
        while let Some(stream) = self.pending_capacity.pop(store) {
            counts.transition(stream, |_, stream| {
                tracing::trace!(?stream.id, "clear_pending_capacity");
            })
        }
    }

    pub fn assign_connection_capacity<R>(
        &mut self,
        inc: WindowSize,
        store: &mut R,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) where
        R: Resolve,
    {
        let span = tracing::trace_span!("assign_connection_capacity", inc);
        let _e = span.enter();

        // TODO: proper error handling
        let _res = self.flow.assign_capacity(inc);
        debug_assert!(_res.is_ok());

        // Assign newly acquired capacity to streams pending capacity.
        while self.flow.available() > 0 {
            let stream = match self.pending_capacity.pop(store) {
                Some(stream) => stream,
                None => return,
            };

            // Streams pending capacity may have been reset before capacity
            // became available. In that case, the stream won't want any
            // capacity, and so we shouldn't "transition" on it, but just evict
            // it and continue the loop.
            if !(stream.state.is_send_streaming() || stream.buffered_send_data > 0) {
                continue;
            }

            counts.transition(stream, |_, stream| {
                // Try to assign capacity to the stream. This will also re-queue the
                // stream if there isn't enough connection level capacity to fulfill
                // the capacity request.
                self.try_assign_capacity(stream, task);
            })
        }
    }

    /// Request capacity to send data
    fn try_assign_capacity(&mut self, stream: &mut store::Ptr, task: &mut Option<Waker>) {
        // Streams over the max concurrent count should not have capacity assign to avoid starving the connection
        // capacity for open streams. pending_push is the same: PUSH_PROMISE is
        // not flow-controlled, but the child may then be `queue_open`'d if the
        // send slot is already taken (F91). Assigned capacity on pending_open
        // starves every stream that can actually send.
        if stream.is_pending_open || stream.is_pending_push {
            return;
        }

        let total_requested = stream.requested_send_capacity;

        // Total requested should never go below actual assigned
        // (Note: the window size can go lower than assigned after SETTINGS)
        debug_assert!(stream.send_flow.available() <= total_requested as usize);

        // Additional capacity to assign: limited by request and peer window.
        // Use saturating_sub: when available already exceeds window (or request),
        // plain u32 subtraction wraps and can over-claim connection capacity.
        let additional = additional_send_capacity(
            total_requested,
            stream.send_flow.available().as_size(),
            stream.send_flow.window_size(),
        );
        let span = tracing::trace_span!("try_assign_capacity", ?stream.id);
        let _e = span.enter();
        tracing::trace!(
            requested = total_requested,
            additional,
            buffered = stream.buffered_send_data,
            window = stream.send_flow.window_size(),
            conn = %self.flow.available()
        );

        if additional == 0 {
            // Nothing more to do
            return;
        }

        // The stream may have been reset or closed since capacity was requested.
        if !stream.state.is_send_streaming() && stream.buffered_send_data == 0 {
            return;
        }

        // The amount of currently available capacity on the connection
        let conn_available = self.flow.available().as_size();

        // First check if capacity is immediately available
        if conn_available > 0 {
            // The amount of capacity to assign to the stream
            // TODO: Should prioritization factor into this?
            let assign = cmp::min(conn_available, additional);

            tracing::trace!(capacity = assign, "assigning");

            // Assign the capacity to the stream
            stream.assign_capacity(assign, self.max_buffer_size);

            // Claim the capacity from the connection
            // TODO: proper error handling
            let _res = self.flow.claim_capacity(assign);
            debug_assert!(_res.is_ok());
        }

        tracing::trace!(
            available = %stream.send_flow.available(),
            requested = stream.requested_send_capacity,
            buffered = stream.buffered_send_data,
            has_unavailable = %stream.send_flow.has_unavailable()
        );

        if stream.send_flow.available() < stream.requested_send_capacity as usize
            && stream.send_flow.has_unavailable()
        {
            // The stream requires additional capacity and the stream's
            // window has available capacity, but the connection window
            // does not.
            //
            // In this case, the stream needs to be queued up for when the
            // connection has more capacity.
            self.pending_capacity.push(stream);
        }

        // If data is buffered and the stream is send ready, then
        // schedule the stream for execution
        if stream.buffered_send_data > 0 && stream.is_send_ready() {
            // TODO: This assertion isn't *exactly* correct. There can still be
            // buffered send data while the stream's pending send queue is
            // empty. This can happen when a large data frame is in the process
            // of being **partially** sent. Once the window has been sent, the
            // data frame will be returned to the prioritization layer to be
            // re-scheduled.
            //
            // That said, it would be nice to figure out how to make this
            // assertion correctly.
            //
            // debug_assert!(!stream.pending_send.is_empty());

            self.pending_send.push(stream);
            // User-thread reclaim (`reserve_capacity` decrease) can schedule
            // another stream while the connection is parked on read.
            if let Some(task) = task.take() {
                task.wake();
            }
        }
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
        // Reclaim any frame that has previously been written
        self.reclaim_frame(buffer, store, dst);

        // The max frame length
        let max_frame_len = dst.max_send_frame_size();

        tracing::trace!("buffer_pending");
        self.debug_assert_send_capacity_conservation(store);

        loop {
            if !dst.has_send_capacity() {
                self.debug_assert_send_capacity_conservation(store);
                return Ok(BufferStatus::CodecFull);
            }

            // Drop cancelled / reset streams that never left pending_open.
            // They were never opened on the wire, so HEADERS+RST must not be
            // sent (RST on idle is PROTOCOL_ERROR). They also must not wait
            // for a concurrency slot: with MAX_CONCURRENT_STREAMS=0 they would
            // leak forever.
            while self.abort_closed_pending_open(buffer, store, counts) {}

            if let Some(mut stream) = self.pop_pending_open(store, counts) {
                self.pending_send.push_front(&mut stream);
                self.try_assign_capacity(&mut stream, &mut None);
            }

            match self.pop_frame(buffer, store, max_frame_len, counts) {
                Some(frame) => {
                    tracing::trace!(?frame, "writing");

                    debug_assert_eq!(self.in_flight_data_frame, InFlightData::Nothing);
                    if let Frame::Data(ref frame) = frame {
                        self.in_flight_data_frame = InFlightData::DataFrame(frame.payload().stream);
                    }
                    dst.buffer(frame).expect("invalid frame");

                    // Small DATA frames can be fully encoded by `buffer`,
                    // which records completion in a single codec slot. Reclaim
                    // before accepting another frame so that slot is not
                    // overwritten.
                    self.reclaim_frame(buffer, store, dst);
                }
                None => {
                    self.debug_assert_send_capacity_conservation(store);
                    return Ok(BufferStatus::Complete);
                }
            }
        }
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
        self.reclaim_frame(buffer, store, dst)
    }

    /// Tries to reclaim a pending data frame from the codec.
    ///
    /// Returns true if a frame was reclaimed.
    ///
    /// When a data frame is written to the codec, it may not be written in its
    /// entirety (large chunks are split up into potentially many data frames).
    /// In this case, the stream needs to be reprioritized.
    fn reclaim_frame<T, B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        dst: &mut Codec<T, Prioritized<B>>,
    ) -> bool
    where
        B: Buf,
    {
        let span = tracing::trace_span!("try_reclaim_frame");
        let _e = span.enter();

        // First check if there are any data chunks to take back
        if let Some(frame) = dst.take_last_data_frame() {
            self.reclaim_frame_inner(buffer, store, frame)
        } else {
            false
        }
    }

    fn reclaim_frame_inner<B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        frame: frame::Data<Prioritized<B>>,
    ) -> bool
    where
        B: Buf,
    {
        tracing::trace!(
            ?frame,
            sz = frame.payload().inner.get_ref().remaining(),
            "reclaimed"
        );

        let mut eos = false;
        let key = frame.payload().stream;

        match mem::replace(&mut self.in_flight_data_frame, InFlightData::Nothing) {
            InFlightData::Nothing => panic!("wasn't expecting a frame to reclaim"),
            InFlightData::Drop => {
                tracing::trace!("not reclaiming frame for cancelled stream");
                return false;
            }
            InFlightData::DataFrame(k) => {
                debug_assert_eq!(k, key);
            }
        }

        let mut frame = frame.map(|prioritized| {
            // TODO: Ensure fully written
            eos = prioritized.end_of_stream;
            prioritized.inner.into_inner()
        });

        if frame.payload().has_remaining() {
            let mut stream = store.resolve(key);

            if eos {
                frame.set_end_stream(true);
            }

            self.push_back_frame(frame.into(), buffer, &mut stream);

            return true;
        }

        false
    }

    /// Push the frame to the front of the stream's deque, scheduling the
    /// stream if needed.
    fn push_back_frame<B>(
        &mut self,
        frame: Frame<B>,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
    ) {
        // Push the frame to the front of the stream's deque
        stream.pending_send.push_front(buffer, frame);

        // If needed, schedule the sender
        if stream.send_flow.available() > 0 {
            debug_assert!(!stream.pending_send.is_empty());
            self.pending_send.push(stream);
        } else if stream.send_flow.has_unavailable() {
            // Remainder of a partially written DATA frame with no assigned
            // capacity: wait for connection capacity (stream window still open).
            self.pending_capacity.push(stream);
        }
    }

    pub fn clear_queue<B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        stream: &mut store::Ptr,
        counts: &mut Counts,
        task: &mut Option<Waker>,
    ) {
        let span = tracing::trace_span!("clear_queue", ?stream.id);
        let _e = span.enter();

        // TODO: make this more efficient?
        while let Some(frame) = stream.pending_send.pop_front(buffer) {
            tracing::trace!(?frame, "dropping");
            // PUSH_PROMISE never left the queue: the promised stream was never
            // reserved on the wire. Free it locally (no RST — peer never saw it).
            if let Frame::PushPromise(ref pp) = frame {
                let promised_id = pp.promised_id();
                if let Some(mut pushed) = stream.store_mut().find_mut(&promised_id) {
                    tracing::trace!(
                        "clear_queue; discard never-sent promised stream={:?}",
                        promised_id
                    );
                    pushed.is_pending_push = false;
                    // Nested frames on the child should not include another PP
                    // before its own PP was sent; clear without re-entry.
                    while let Some(_) = pushed.pending_send.pop_front(buffer) {}
                    pushed.buffered_send_data = 0;
                    pushed.requested_send_capacity = 0;
                    // try_assign does not skip pending_push, so the child may
                    // already hold connection send capacity. Zeroing buffered
                    // without reclaiming starves every other stream (F90).
                    self.reclaim_all_capacity(&mut pushed, counts, task);
                    if !pushed.state.is_closed() {
                        pushed.set_reset(Reason::CANCEL, Initiator::Library);
                    } else if let Some(reason) = pushed.state.get_scheduled_reset() {
                        pushed.set_reset(reason, Initiator::Library);
                    }
                    let is_pending_reset = pushed.is_pending_reset_expiration();
                    counts.transition_after(pushed, is_pending_reset);
                }
            }
        }

        stream.buffered_send_data = 0;
        stream.requested_send_capacity = 0;
        if let InFlightData::DataFrame(key) = self.in_flight_data_frame {
            if stream.key() == key {
                // This stream could get cleaned up now - don't allow the buffered frame to get reclaimed.
                self.in_flight_data_frame = InFlightData::Drop;
            }
        }
    }

    pub fn clear_pending_send(&mut self, store: &mut Store, counts: &mut Counts) {
        while let Some(mut stream) = self.pending_send.pop(store) {
            let is_pending_reset = stream.is_pending_reset_expiration();
            if let Some(reason) = stream.state.get_scheduled_reset() {
                stream.set_reset(reason, Initiator::Library);
            }
            counts.transition_after(stream, is_pending_reset);
        }
    }

    pub fn clear_pending_open(&mut self, store: &mut Store, counts: &mut Counts) {
        while let Some(stream) = self.pending_open.pop(store) {
            counts.dec_num_pending_open();
            let is_pending_reset = stream.is_pending_reset_expiration();
            counts.transition_after(stream, is_pending_reset);
        }
    }

    fn pop_frame<B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        max_len: usize,
        counts: &mut Counts,
    ) -> Option<Frame<Prioritized<B>>>
    where
        B: Buf,
    {
        let span = tracing::trace_span!("pop_frame");
        let _e = span.enter();

        loop {
            match self.pending_send.pop(store) {
                Some(mut stream) => {
                    let span = tracing::trace_span!("popped", ?stream.id, ?stream.state);
                    let _e = span.enter();

                    // It's possible that this stream, besides having data to send,
                    // is also queued to send a reset, and thus is already in the queue
                    // to wait for "some time" after a reset.
                    //
                    // To be safe, we just always ask the stream.
                    let is_pending_reset = stream.is_pending_reset_expiration();

                    tracing::trace!(is_pending_reset);

                    let frame = match stream.pending_send.pop_front(buffer) {
                        Some(Frame::Data(mut frame)) => {
                            if let Some(reason) = stream.state.get_scheduled_reset() {
                                // If a reset is scheduled due to cancellation or
                                // an error, discard buffered DATA and let the `None`
                                // arm emit the RST_STREAM on the next iteration.
                                //
                                // NO_ERROR is excluded. Per RFC 9113 §8.1, a NO_ERROR
                                // stream reset may only be sent after a complete
                                // response, which requires sending all queued DATA.
                                // (If the stream window was already 0 at schedule
                                // time, maybe_cancel uses CANCEL instead — see F30.)
                                if reason != Reason::NO_ERROR {
                                    stream.pending_send.push_front(buffer, frame.into());
                                    self.clear_queue(buffer, &mut stream, counts, &mut None);
                                    self.reclaim_all_capacity(&mut stream, counts, &mut None);
                                    self.pending_send.push(&mut stream);
                                    continue;
                                }
                            }

                            // Get the amount of capacity remaining for stream's
                            // window.
                            let stream_capacity = stream.send_flow.available();
                            let sz = frame.payload().remaining();

                            tracing::trace!(
                                sz,
                                eos = frame.is_end_stream(),
                                window = %stream_capacity,
                                available = %stream.send_flow.available(),
                                requested = stream.requested_send_capacity,
                                buffered = stream.buffered_send_data,
                                "data frame"
                            );

                            // Zero length data frames always have capacity to
                            // be sent.
                            if sz > 0 && stream_capacity == 0 {
                                tracing::trace!("stream capacity is 0");

                                // The stream has no more capacity, this can
                                // happen if the remote reduced the stream
                                // window or connection capacity was reclaimed
                                // after the stream was scheduled. Buffer the
                                // frame and wait for a window update.
                                //
                                // Ensure we are in `pending_capacity` when more
                                // connection capacity would help; otherwise a
                                // later connection WINDOW_UPDATE will never
                                // re-schedule this stream (S3).
                                stream.pending_send.push_front(buffer, frame.into());
                                if stream.send_flow.has_unavailable() {
                                    self.pending_capacity.push(&mut stream);
                                }

                                continue;
                            }

                            // Only send up to the max frame length
                            let len = cmp::min(sz, max_len);

                            // Only send up to the stream's window capacity
                            let len =
                                cmp::min(len, stream_capacity.as_size() as usize) as WindowSize;

                            // There *must* be be enough connection level
                            // capacity at this point.
                            debug_assert!(len <= self.flow.window_size());

                            // Check if the stream level window the peer knows is available. In some
                            // scenarios, maybe the window we know is available but the window which
                            // peer knows is not.
                            if len > 0 && len > stream.send_flow.window_size() {
                                stream.pending_send.push_front(buffer, frame.into());
                                // Same as the capacity==0 path: do not leave the
                                // stream off both send and capacity queues.
                                if stream.send_flow.has_unavailable() {
                                    self.pending_capacity.push(&mut stream);
                                }
                                continue;
                            }

                            tracing::trace!(len, "sending data frame");

                            // Update the flow control
                            tracing::trace_span!("updating stream flow").in_scope(|| {
                                stream.send_data(len, self.max_buffer_size);

                                // Assign the capacity back to the connection that
                                // was just consumed from the stream in the previous
                                // line.
                                // TODO: proper error handling
                                let _res = self.flow.assign_capacity(len);
                                debug_assert!(_res.is_ok());
                            });

                            let (eos, len) = tracing::trace_span!("updating connection flow")
                                .in_scope(|| {
                                    // TODO: proper error handling
                                    let _res = self.flow.send_data(len);
                                    debug_assert!(_res.is_ok());

                                    // Wrap the frame's data payload to ensure that the
                                    // correct amount of data gets written.

                                    let eos = frame.is_end_stream();
                                    let len = len as usize;

                                    if frame.payload().remaining() > len {
                                        frame.set_end_stream(false);
                                    }
                                    (eos, len)
                                });

                            Frame::Data(frame.map(|buf| Prioritized {
                                inner: buf.take(len),
                                end_of_stream: eos,
                                stream: stream.key(),
                            }))
                        }
                        Some(Frame::PushPromise(pp)) => {
                            let mut pushed =
                                stream.store_mut().find_mut(&pp.promised_id()).unwrap();
                            pushed.is_pending_push = false;
                            // PUSH_PROMISE is now on the wire: the promised stream is
                            // reserved at the peer. Schedule send if headers were
                            // queued, or if the child was cancelled while still
                            // pending_push (schedule_send is a no-op while
                            // is_pending_push — RST would never leave otherwise).
                            if pushed.state.is_scheduled_reset() {
                                self.pending_send.push(&mut pushed);
                            } else if !pushed.pending_send.is_empty() {
                                // Transition stream from pending_push to open /
                                // pending_open if possible
                                if counts.can_inc_num_send_streams() {
                                    counts.inc_num_send_streams(&mut pushed);
                                    // Capacity was not assigned while pending_push
                                    // (try_assign skips it). Assign now that the
                                    // child can send, or DATA sits at capacity 0
                                    // with the connection window unused.
                                    self.try_assign_capacity(&mut pushed, &mut None);
                                    self.pending_send.push(&mut pushed);
                                } else {
                                    // Defense: never take assigned capacity into
                                    // pending_open (I1). No-op if try_assign
                                    // already skipped pending_push.
                                    self.reclaim_all_capacity(&mut pushed, counts, &mut None);
                                    self.queue_open(&mut pushed, counts);
                                }
                            }
                            Frame::PushPromise(pp)
                        }
                        Some(frame) => frame.map(|_| {
                            unreachable!(
                                "Frame::map closure will only be called \
                                 on DATA frames."
                            )
                        }),
                        None => {
                            if let Some(reason) = stream.state.get_scheduled_reset() {
                                stream.set_reset(reason, Initiator::Library);

                                let frame = frame::Reset::new(stream.id, reason);
                                Frame::Reset(frame)
                            } else {
                                // If the stream receives a RESET from the peer, it may have
                                // had data buffered to be sent, but all the frames are cleared
                                // in clear_queue(). Instead of doing O(N) traversal through queue
                                // to remove, lets just ignore the stream here.
                                tracing::trace!("removing dangling stream from pending_send");
                                // Since this should only happen as a consequence of `clear_queue`,
                                // we must be in a closed state of some kind.
                                debug_assert!(stream.state.is_closed());
                                counts.transition_after(stream, is_pending_reset);
                                continue;
                            }
                        }
                    };

                    tracing::trace!("pop_frame; frame={:?}", frame);

                    if cfg!(debug_assertions) && stream.state.is_idle() {
                        debug_assert!(stream.id > self.last_opened_id);
                        self.last_opened_id = stream.id;
                    }

                    if !stream.pending_send.is_empty() || stream.state.is_scheduled_reset() {
                        // TODO: Only requeue the sender IF it is ready to send
                        // the next frame. i.e. don't requeue it if the next
                        // frame is a data frame and the stream does not have
                        // any more capacity.
                        self.pending_send.push(&mut stream);
                    }

                    counts.transition_after(stream, is_pending_reset);

                    return Some(frame);
                }
                None => return None,
            }
        }
    }

    fn pop_pending_open<'s>(
        &mut self,
        store: &'s mut Store,
        counts: &mut Counts,
    ) -> Option<store::Ptr<'s>> {
        tracing::trace!("schedule_pending_open");
        // check for any pending open streams
        if counts.can_inc_num_send_streams() {
            if let Some(mut stream) = self.pending_open.pop(store) {
                tracing::trace!("schedule_pending_open; stream={:?}", stream.id);

                counts.dec_num_pending_open();
                counts.inc_num_send_streams(&mut stream);
                // Wake both: SendRequest may wait on open_task while SendStream
                // poll_capacity/poll_reset wait on send_task.
                stream.notify_open();
                stream.notify_send();
                return Some(stream);
            }
        }

        None
    }

    /// Remove never-sent streams from `pending_open` when they can never open
    /// (or were already cancelled/reset).
    ///
    /// - Implicit cancel (`ScheduledLibraryReset`): user dropped all handles.
    /// - Explicit `send_reset` with empty `pending_send`: discarded because no
    ///   concurrency slot was available (HEADERS+RST would never flush).
    /// - Any stream when `max_send_streams == 0`: a slot can never open (peer
    ///   SETTINGS or post-queue max decrease). Healthy streams are reset with
    ///   `REFUSED_STREAM` so waiters fail instead of hanging forever.
    ///
    /// Server `pending_open` is different: PUSH_PROMISE already advertised the
    /// id. Cancel/reset/max=0 emit RST without a concurrency slot (F93).
    ///
    /// Explicit reset that still has HEADERS+RST queued is left alone when a
    /// slot may still open later (avoids RST on idle), unless max is 0.
    ///
    /// Scans the **entire** queue (not only the head). A cancelled stream
    /// buried behind a healthy pending_open head would otherwise leak for as
    /// long as the head cannot open (e.g. max concurrent still full).
    ///
    /// Returns true if any stream was aborted (caller may loop).
    fn abort_closed_pending_open<B>(
        &mut self,
        buffer: &mut Buffer<Frame<B>>,
        store: &mut Store,
        counts: &mut Counts,
    ) -> bool {
        let max_zero = counts.max_send_streams() == 0;
        // Rebuild the queue so non-head abortable streams are not stranded.
        let mut keep = store::Queue::<stream::NextOpen>::new();
        let mut aborted = false;

        while let Some(mut stream) = self.pending_open.pop(store) {
            counts.dec_num_pending_open();
            let should_abort = max_zero
                || stream.state.is_scheduled_reset()
                || (stream.state.is_reset() && stream.pending_send.is_empty());

            if !should_abort {
                keep.push(&mut stream);
                continue;
            }

            tracing::trace!(
                "abort_closed_pending_open; stream={:?}; state={:?}; max_zero={}",
                stream.id,
                stream.state,
                max_zero
            );

            // Server pending_open is a push whose PUSH_PROMISE already went
            // out (queue_open after PP pop). The peer sees reserved, not idle;
            // RFC §5.1 allows RST without a MAX_CONCURRENT_STREAMS slot.
            // Local-only abort would leak a reserved stream at the peer (F93).
            if counts.peer().is_server() {
                let reason = stream
                    .state
                    .reset_reason()
                    .unwrap_or(Reason::REFUSED_STREAM);
                self.clear_queue(buffer, &mut stream, counts, &mut None);
                self.reclaim_all_capacity(&mut stream, counts, &mut None);
                if stream.state.get_scheduled_reset().is_some() {
                    self.pending_send.push(&mut stream);
                } else {
                    if !stream.state.is_reset() {
                        stream.state.set_scheduled_reset(reason);
                    }
                    if stream.state.get_scheduled_reset().is_some() {
                        self.pending_send.push(&mut stream);
                    } else {
                        let frame = frame::Reset::new(stream.id, reason);
                        self.queue_frame(frame.into(), buffer, &mut stream, &mut None);
                    }
                }
                aborted = true;
                continue;
            }

            // Never opened on the wire: discard any leftover frames and release.
            self.clear_queue(buffer, &mut stream, counts, &mut None);
            self.reclaim_all_capacity(&mut stream, counts, &mut None);
            if let Some(reason) = stream.state.get_scheduled_reset() {
                stream.set_reset(reason, Initiator::Library);
            } else if !stream.state.is_reset() {
                // Healthy stream that can never open (max concurrent is 0).
                stream.set_reset(Reason::REFUSED_STREAM, Initiator::Library);
            }
            let is_pending_reset = stream.is_pending_reset_expiration();
            counts.transition_after(stream, is_pending_reset);
            aborted = true;
        }

        // Preserve FIFO order of streams that still wait to open.
        while let Some(mut stream) = keep.pop(store) {
            self.queue_open(&mut stream, counts);
        }

        aborted
    }
}

// ===== impl Prioritized =====

impl<B> Buf for Prioritized<B>
where
    B: Buf,
{
    fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.inner.chunk()
    }

    fn chunks_vectored<'a>(&'a self, dst: &mut [std::io::IoSlice<'a>]) -> usize {
        self.inner.chunks_vectored(dst)
    }

    fn advance(&mut self, cnt: usize) {
        self.inner.advance(cnt)
    }
}

impl<B: Buf> fmt::Debug for Prioritized<B> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Prioritized")
            .field("remaining", &self.inner.get_ref().remaining())
            .field("end_of_stream", &self.end_of_stream)
            .field("stream", &self.stream)
            .finish()
    }
}

/// How much more connection capacity may be assigned to a stream.
///
/// `available` is already assigned to the stream; `window` is the peer stream
/// window (`as_size`, so negative peer windows are 0). Saturating arithmetic
/// avoids u32 wrap when available already exceeds window or requested (e.g.
/// briefly after SETTINGS decrease before reclaim).
fn additional_send_capacity(
    requested: WindowSize,
    available: WindowSize,
    window: WindowSize,
) -> WindowSize {
    cmp::min(
        requested.saturating_sub(available),
        window.saturating_sub(available),
    )
}

#[cfg(test)]
mod additional_send_capacity_tests {
    use super::*;

    #[test]
    fn assigns_up_to_request_and_window() {
        assert_eq!(additional_send_capacity(100, 20, 80), 60);
        assert_eq!(additional_send_capacity(50, 20, 80), 30);
        assert_eq!(additional_send_capacity(100, 0, 0), 0);
    }

    #[test]
    fn zero_when_available_exceeds_window() {
        // Pre-fix: 0u32 - 10 wraps to ~4e9 and would over-assign.
        assert_eq!(additional_send_capacity(100, 10, 0), 0);
        assert_eq!(additional_send_capacity(100, 50, 40), 0);
    }

    #[test]
    fn zero_when_available_exceeds_requested() {
        assert_eq!(additional_send_capacity(10, 20, 100), 0);
    }
}
