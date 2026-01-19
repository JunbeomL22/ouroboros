//! Error types for Binance WebSocket client

use thiserror::Error;

/// Errors that can occur when using the Binance WebSocket client
#[derive(Error, Debug)]
pub enum BinanceError {
    /// WebSocket protocol error
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Connection was closed by the server
    #[error("Connection closed: code={code}, reason={reason}")]
    ConnectionClosed { code: u16, reason: String },

    /// Maximum reconnection attempts exceeded
    #[error("Maximum reconnection retries exceeded")]
    MaxRetriesExceeded,

    /// Subscription request failed
    #[error("Subscription failed: id={id}, code={code}, msg={msg}")]
    SubscriptionFailed { id: u64, code: i64, msg: String },

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(String),

    /// Channel send error
    #[error("Channel send error")]
    ChannelSend,

    /// Connection timeout
    #[error("Connection timeout")]
    ConnectionTimeout,
}

/// Result type alias for BinanceError
pub type Result<T> = std::result::Result<T, BinanceError>;

/// Determines reconnection behavior based on WebSocket close code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Normal closure, no reconnection needed
    NoReconnect,
    /// Reconnect immediately
    ReconnectImmediate,
    /// Reconnect with exponential backoff
    ReconnectWithBackoff,
}

impl CloseAction {
    /// Determine the appropriate action based on WebSocket close code
    pub fn from_close_code(code: u16) -> Self {
        match code {
            1000 => CloseAction::NoReconnect,           // Normal closure
            1001 => CloseAction::ReconnectWithBackoff,  // Going away
            1006 => CloseAction::ReconnectImmediate,    // Abnormal closure
            1008 => CloseAction::ReconnectWithBackoff,  // Policy violation (server overload)
            1011 => CloseAction::ReconnectWithBackoff,  // Unexpected condition
            1013 => CloseAction::ReconnectWithBackoff,  // Try again later
            _ => CloseAction::ReconnectWithBackoff,
        }
    }
}
