//! Tokio glue for `mdstream`.
//!
//! `mdstream` is runtime-agnostic and is intended to be owned by a UI thread (single-owner).
//! This crate provides small helpers for async producers:
//!
//! - Coalesce tiny deltas into larger chunks (newline-gated and/or time-window flush).
//! - Optionally run an actor task that owns `MdStream` and emits owned `Update`s.
//!
//! For a full TUI example, see `cargo run -p mdstream-tokio --example agent_tui`.

use mdstream::MdStream;
use mdstream::Update;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

#[derive(Clone, Copy, Debug)]
pub struct CoalesceOptions {
    /// Flush once a newline is observed in the buffered text.
    pub flush_on_newline: bool,
    /// Flush if no flush happened for this duration (progress guarantee).
    pub max_delay: Duration,
    /// Flush when buffered bytes reach this limit.
    pub max_bytes: usize,
}

impl Default for CoalesceOptions {
    fn default() -> Self {
        Self {
            flush_on_newline: true,
            max_delay: Duration::from_millis(60),
            max_bytes: 8 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CoalescePreset {
    Balanced,
    Fast,
    TimeOnly,
}

impl CoalescePreset {
    pub fn next(self) -> Self {
        match self {
            CoalescePreset::Balanced => CoalescePreset::Fast,
            CoalescePreset::Fast => CoalescePreset::TimeOnly,
            CoalescePreset::TimeOnly => CoalescePreset::Balanced,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CoalescePreset::Balanced => "balanced",
            CoalescePreset::Fast => "fast",
            CoalescePreset::TimeOnly => "time-only",
        }
    }

    pub fn options(self) -> CoalesceOptions {
        match self {
            CoalescePreset::Balanced => CoalesceOptions {
                flush_on_newline: true,
                max_delay: Duration::from_millis(80),
                max_bytes: 16 * 1024,
            },
            CoalescePreset::Fast => CoalesceOptions {
                flush_on_newline: true,
                max_delay: Duration::from_millis(30),
                max_bytes: 4 * 1024,
            },
            CoalescePreset::TimeOnly => CoalesceOptions {
                flush_on_newline: false,
                max_delay: Duration::from_millis(60),
                max_bytes: 4 * 1024,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackpressurePolicy {
    /// Await capacity. Never drops.
    ///
    /// Recommended when:
    /// - you need reliable delivery (no content loss)
    /// - your producer can tolerate waiting (e.g. network stream on a background task)
    ///
    /// Trade-off: the producer task may stall when the UI falls behind.
    Block,
    /// Drop the new delta when the channel is full.
    ///
    /// Recommended when:
    /// - deltas are replaceable / “best effort” (typing indicators, progress, ephemeral status)
    /// - you prefer keeping the UI responsive over preserving every update
    ///
    /// Trade-off: content loss is expected when the UI is slow.
    DropNew,
    /// Buffer locally and try to flush opportunistically (keeps content, reduces producer stalls).
    ///
    /// This is useful when producers are very “bursty” and you prefer UI smoothness over strict
    /// per-token delivery. It combines well with a receiver-side coalescer.
    ///
    /// Recommended when:
    /// - deltas are very high-frequency (LLM token streams)
    /// - you still want to preserve content, but avoid stalling producers on every small chunk
    ///
    /// Trade-off: memory is bounded by `local_max_bytes`; flushing becomes “chunky” under load.
    CoalesceLocal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Sent,
    Dropped,
    Buffered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendError {
    Closed,
}

/// Producer-side helper for bounded channels.
///
/// In many streaming setups, the producer runs in an async task and the UI thread drains updates.
/// This wrapper provides a few practical backpressure strategies without forcing users to build
/// their own channel policies.
pub struct DeltaSender {
    tx: mpsc::Sender<String>,
    policy: BackpressurePolicy,
    local_buf: String,
    local_max_bytes: usize,
}

impl DeltaSender {
    pub fn new(tx: mpsc::Sender<String>, policy: BackpressurePolicy) -> Self {
        Self {
            tx,
            policy,
            local_buf: String::new(),
            local_max_bytes: 16 * 1024,
        }
    }

    pub fn set_local_max_bytes(&mut self, max: usize) {
        self.local_max_bytes = max.max(1);
    }

    pub fn policy(&self) -> BackpressurePolicy {
        self.policy
    }

    pub fn set_policy(&mut self, policy: BackpressurePolicy) {
        self.policy = policy;
    }

    pub async fn send(&mut self, delta: &str) -> Result<SendOutcome, SendError> {
        match self.policy {
            BackpressurePolicy::Block => self.send_block(delta).await,
            BackpressurePolicy::DropNew => self.send_drop_new(delta),
            BackpressurePolicy::CoalesceLocal => self.send_coalesce_local(delta).await,
        }
    }

    pub async fn flush(&mut self) -> Result<SendOutcome, SendError> {
        if self.local_buf.is_empty() {
            return Ok(SendOutcome::Sent);
        }
        let buf = std::mem::take(&mut self.local_buf);
        self.tx.send(buf).await.map_err(|_| SendError::Closed)?;
        Ok(SendOutcome::Sent)
    }

    async fn send_block(&mut self, delta: &str) -> Result<SendOutcome, SendError> {
        self.tx
            .send(delta.to_string())
            .await
            .map_err(|_| SendError::Closed)?;
        Ok(SendOutcome::Sent)
    }

    fn send_drop_new(&mut self, delta: &str) -> Result<SendOutcome, SendError> {
        match self.tx.try_send(delta.to_string()) {
            Ok(()) => Ok(SendOutcome::Sent),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Ok(SendOutcome::Dropped),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => Err(SendError::Closed),
        }
    }

    async fn send_coalesce_local(&mut self, delta: &str) -> Result<SendOutcome, SendError> {
        self.local_buf.push_str(delta);

        let should_try_flush =
            self.local_buf.len() >= self.local_max_bytes || self.local_buf.contains('\n');

        if should_try_flush {
            match self.tx.try_send(std::mem::take(&mut self.local_buf)) {
                Ok(()) => return Ok(SendOutcome::Sent),
                Err(tokio::sync::mpsc::error::TrySendError::Full(s)) => {
                    self.local_buf = s;
                    return Ok(SendOutcome::Buffered);
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    return Err(SendError::Closed)
                }
            }
        }

        Ok(SendOutcome::Buffered)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlushReason {
    Newline,
    MaxDelay,
    MaxBytes,
    ChannelClosed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoalescedChunk {
    pub text: String,
    pub reason: FlushReason,
    /// Number of input messages merged into this output chunk.
    pub merged_messages: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoalesceStats {
    pub total_in_messages: u64,
    pub total_out_chunks: u64,
    pub total_out_bytes: u64,
    pub last_reason: Option<FlushReason>,
    pub last_merged_messages: usize,
    pub last_bytes: usize,
}

/// A receiver wrapper that merges high-frequency deltas into fewer, larger chunks.
pub struct CoalescingReceiver {
    rx: mpsc::Receiver<String>,
    opts: CoalesceOptions,
    buf: String,
    deadline: Option<Instant>,
    stats: CoalesceStats,
}

impl CoalescingReceiver {
    pub fn new(rx: mpsc::Receiver<String>, opts: CoalesceOptions) -> Self {
        Self {
            rx,
            opts,
            buf: String::new(),
            deadline: None,
            stats: CoalesceStats::default(),
        }
    }

    pub fn set_options(&mut self, opts: CoalesceOptions) {
        self.opts = opts;
        // Keep any buffered text; refresh the deadline based on the new policy.
        if !self.buf.is_empty() {
            self.deadline = Some(Instant::now() + self.opts.max_delay);
        }
    }

    pub fn options(&self) -> CoalesceOptions {
        self.opts
    }

    pub fn stats(&self) -> CoalesceStats {
        self.stats
    }

    /// Receive the next coalesced chunk.
    ///
    /// - Returns `None` when the underlying channel is closed and the internal buffer is empty.
    /// - Returns a final buffered chunk before finishing, if any.
    pub async fn recv(&mut self) -> Option<String> {
        self.recv_with_meta().await.map(|c| c.text)
    }

    pub async fn recv_with_meta(&mut self) -> Option<CoalescedChunk> {
        let mut merged_messages = 0usize;

        if self.buf.is_empty() {
            let first = self.rx.recv().await?;
            self.buf.push_str(&first);
            merged_messages += 1;
            self.deadline = Some(Instant::now() + self.opts.max_delay);
        }

        loop {
            if let Some(reason) = self.should_flush_reason() {
                let text = self.take_buf();
                self.stats.total_in_messages = self
                    .stats
                    .total_in_messages
                    .saturating_add(merged_messages as u64);
                self.stats.total_out_chunks = self.stats.total_out_chunks.saturating_add(1);
                self.stats.total_out_bytes = self.stats.total_out_bytes.saturating_add(text.len() as u64);
                self.stats.last_reason = Some(reason);
                self.stats.last_merged_messages = merged_messages;
                self.stats.last_bytes = text.len();
                return Some(CoalescedChunk {
                    text,
                    reason,
                    merged_messages,
                });
            }

            let Some(deadline) = self.deadline else {
                self.deadline = Some(Instant::now() + self.opts.max_delay);
                continue;
            };

            let next = tokio::time::timeout_at(deadline, self.rx.recv()).await;
            match next {
                Ok(Some(s)) => {
                    self.buf.push_str(&s);
                    merged_messages += 1;
                }
                Ok(None) => {
                    // Channel closed: flush remaining buffer once.
                    if self.buf.is_empty() {
                        return None;
                    }
                    let reason = FlushReason::ChannelClosed;
                    let text = self.take_buf();
                    self.stats.total_in_messages = self
                        .stats
                        .total_in_messages
                        .saturating_add(merged_messages as u64);
                    self.stats.total_out_chunks = self.stats.total_out_chunks.saturating_add(1);
                    self.stats.total_out_bytes = self.stats.total_out_bytes.saturating_add(text.len() as u64);
                    self.stats.last_reason = Some(reason);
                    self.stats.last_merged_messages = merged_messages;
                    self.stats.last_bytes = text.len();
                    return Some(CoalescedChunk {
                        text,
                        reason,
                        merged_messages,
                    });
                }
                Err(_) => {
                    // Timeout: flush for progress.
                    let reason = FlushReason::MaxDelay;
                    let text = self.take_buf();
                    self.stats.total_in_messages = self
                        .stats
                        .total_in_messages
                        .saturating_add(merged_messages as u64);
                    self.stats.total_out_chunks = self.stats.total_out_chunks.saturating_add(1);
                    self.stats.total_out_bytes = self.stats.total_out_bytes.saturating_add(text.len() as u64);
                    self.stats.last_reason = Some(reason);
                    self.stats.last_merged_messages = merged_messages;
                    self.stats.last_bytes = text.len();
                    return Some(CoalescedChunk {
                        text,
                        reason,
                        merged_messages,
                    });
                }
            }
        }
    }

    fn should_flush_reason(&self) -> Option<FlushReason> {
        if self.buf.len() >= self.opts.max_bytes {
            return Some(FlushReason::MaxBytes);
        }
        if self.opts.flush_on_newline && self.buf.contains('\n') {
            return Some(FlushReason::Newline);
        }
        None
    }

    fn take_buf(&mut self) -> String {
        self.deadline = None;
        std::mem::take(&mut self.buf)
    }
}

/// Spawn a task that owns `MdStream` and emits owned `Update`s.
///
/// This is useful when your consumer cannot keep `MdStream` on the UI thread, or when you want to
/// isolate parsing work from rendering.
pub fn spawn_mdstream_actor(
    mut stream: MdStream,
    rx: mpsc::Receiver<String>,
    opts: CoalesceOptions,
) -> mpsc::Receiver<Update> {
    let (tx_out, rx_out) = mpsc::channel::<Update>(64);

    tokio::spawn(async move {
        let mut rx = CoalescingReceiver::new(rx, opts);
        while let Some(chunk) = rx.recv().await {
            let u = stream.append(&chunk);
            if tx_out.send(u).await.is_err() {
                return;
            }
        }
        let u = stream.finalize();
        let _ = tx_out.send(u).await;
    });

    rx_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coalesces_until_newline_by_default() {
        let (tx, rx) = mpsc::channel::<String>(8);
        let mut cr = CoalescingReceiver::new(rx, CoalesceOptions::default());

        tx.send("he".to_string()).await.unwrap();
        tx.send("llo".to_string()).await.unwrap();
        tx.send("\n".to_string()).await.unwrap();

        let got = cr.recv_with_meta().await.unwrap();
        assert_eq!(got.text, "hello\n");
        assert_eq!(got.reason, FlushReason::Newline);
        assert_eq!(got.merged_messages, 3);

        let stats = cr.stats();
        assert_eq!(stats.total_in_messages, 3);
        assert_eq!(stats.total_out_chunks, 1);
        assert_eq!(stats.last_reason, Some(FlushReason::Newline));
    }

    #[tokio::test]
    async fn delta_sender_drop_new_drops_when_full() {
        let (tx, mut rx) = mpsc::channel::<String>(1);
        let mut s = DeltaSender::new(tx, BackpressurePolicy::DropNew);

        assert_eq!(s.send("a").await.unwrap(), SendOutcome::Sent);
        // Channel is full (receiver not drained yet).
        assert_eq!(s.send("b").await.unwrap(), SendOutcome::Dropped);

        assert_eq!(rx.recv().await.as_deref(), Some("a"));
        drop(s);
        let got = tokio::time::timeout(Duration::from_millis(200), rx.recv())
            .await
            .expect("receiver should complete once all senders are dropped");
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn delta_sender_coalesce_local_flushes_eventually() {
        let (tx, mut rx) = mpsc::channel::<String>(1);
        let mut s = DeltaSender::new(tx, BackpressurePolicy::CoalesceLocal);
        s.set_local_max_bytes(4);

        // Fill channel so try_send will be full.
        s.tx.try_send("x".to_string()).unwrap();

        assert_eq!(s.send("ab").await.unwrap(), SendOutcome::Buffered);
        assert_eq!(s.send("cd").await.unwrap(), SendOutcome::Buffered); // reaches max_bytes, tries, still full

        // Drain one message, then force flush.
        assert_eq!(rx.recv().await.as_deref(), Some("x"));
        assert_eq!(s.flush().await.unwrap(), SendOutcome::Sent);
        assert_eq!(rx.recv().await.as_deref(), Some("abcd"));
    }
}
