//! Subscription state management

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::oneshot;

use crate::error::Result;

/// Manages WebSocket subscriptions and tracks their state
pub struct SubscriptionManager {
    /// Map of subscription ID to stream name
    subscriptions: HashMap<u64, String>,

    /// Pending response handlers
    pending_responses: HashMap<u64, oneshot::Sender<Result<()>>>,

    /// Counter for generating unique subscription IDs
    next_id: AtomicU64,

    /// List of active stream names for reconnection
    active_streams: Vec<String>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            pending_responses: HashMap::new(),
            next_id: AtomicU64::new(1),
            active_streams: Vec::new(),
        }
    }

    /// Generate a new unique subscription ID
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Register a subscription request
    pub fn register_subscription(&mut self, id: u64, stream: String) -> oneshot::Receiver<Result<()>> {
        let (tx, rx) = oneshot::channel();
        self.subscriptions.insert(id, stream.clone());
        self.pending_responses.insert(id, tx);
        rx
    }

    /// Mark a subscription as successful
    pub fn confirm_subscription(&mut self, id: u64) {
        if let Some(stream) = self.subscriptions.get(&id) {
            if !self.active_streams.contains(stream) {
                self.active_streams.push(stream.clone());
            }
        }
        if let Some(tx) = self.pending_responses.remove(&id) {
            let _ = tx.send(Ok(()));
        }
    }

    /// Mark a subscription as failed
    pub fn fail_subscription(&mut self, id: u64, error: crate::error::BinanceError) {
        self.subscriptions.remove(&id);
        if let Some(tx) = self.pending_responses.remove(&id) {
            let _ = tx.send(Err(error));
        }
    }

    /// Remove a subscription by stream name
    pub fn remove_subscription(&mut self, stream: &str) {
        self.active_streams.retain(|s| s != stream);
        self.subscriptions.retain(|_, v| v != stream);
    }

    /// Get all active stream names (for reconnection)
    pub fn active_streams(&self) -> &[String] {
        &self.active_streams
    }

    /// Clear all pending responses (e.g., on disconnect)
    pub fn clear_pending(&mut self) {
        self.pending_responses.clear();
    }

    /// Check if there are any active subscriptions
    pub fn has_subscriptions(&self) -> bool {
        !self.active_streams.is_empty()
    }

    /// Get the count of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.active_streams.len()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
