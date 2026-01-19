//! Connection management and reconnection logic

use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use futures_util::SinkExt;
use tokio::net::TcpStream;

use crate::error::{BinanceError, CloseAction, Result};

/// Configuration for reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,

    /// Maximum delay between reconnection attempts
    pub max_delay: Duration,

    /// Maximum number of reconnection attempts
    pub max_retries: u32,

    /// Jitter percentage (0.0 to 1.0) to add randomness to delays
    pub jitter_percent: f64,

    /// Connection timeout
    pub connect_timeout: Duration,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_retries: 10,
            jitter_percent: 0.1,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

impl ReconnectConfig {
    /// Calculate the delay for a given attempt number using exponential backoff with jitter
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64 * 2.0_f64.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay.as_millis() as f64);

        // Add jitter: delay * (1 ± jitter_percent)
        let jitter_range = capped_delay * self.jitter_percent;
        let jitter = (rand::random::<f64>() * 2.0 - 1.0) * jitter_range;
        let final_delay = (capped_delay + jitter).max(0.0);

        Duration::from_millis(final_delay as u64)
    }
}

/// WebSocket connection wrapper
pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Connection manager handles WebSocket connection lifecycle
pub struct ConnectionManager {
    /// WebSocket URL
    url: String,

    /// Reconnection configuration
    config: ReconnectConfig,

    /// Current reconnection attempt count
    attempt_count: u32,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(url: String, config: ReconnectConfig) -> Self {
        Self {
            url,
            config,
            attempt_count: 0,
        }
    }

    /// Establish a new WebSocket connection
    pub async fn connect(&mut self) -> Result<WsStream> {
        match timeout(self.config.connect_timeout, connect_async(&self.url)).await {
            Ok(Ok((ws, _response))) => {
                self.attempt_count = 0;
                tracing::info!("WebSocket connected to {}", self.url);
                Ok(ws)
            }
            Ok(Err(e)) => Err(BinanceError::WebSocket(e)),
            Err(_) => Err(BinanceError::ConnectionTimeout),
        }
    }

    /// Connect with retry logic
    pub async fn connect_with_retry(&mut self) -> Result<WsStream> {
        loop {
            match self.connect().await {
                Ok(ws) => return Ok(ws),
                Err(e) => {
                    self.attempt_count += 1;

                    if self.attempt_count >= self.config.max_retries {
                        tracing::error!(
                            "Max reconnection attempts ({}) exceeded",
                            self.config.max_retries
                        );
                        return Err(BinanceError::MaxRetriesExceeded);
                    }

                    let delay = self.config.delay_for_attempt(self.attempt_count);
                    tracing::warn!(
                        "Connection failed (attempt {}/{}): {}. Retrying in {:?}",
                        self.attempt_count,
                        self.config.max_retries,
                        e,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Determine if we should reconnect based on close code
    pub fn should_reconnect(&self, code: u16) -> bool {
        match CloseAction::from_close_code(code) {
            CloseAction::NoReconnect => false,
            CloseAction::ReconnectImmediate | CloseAction::ReconnectWithBackoff => true,
        }
    }

    /// Reset attempt count (e.g., after successful message exchange)
    pub fn reset_attempts(&mut self) {
        self.attempt_count = 0;
    }

    /// Get current attempt count
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }
}

/// Send a ping message to keep the connection alive
pub async fn send_ping(ws: &mut WsStream, payload: Vec<u8>) -> Result<()> {
    ws.send(Message::Ping(payload.into())).await?;
    Ok(())
}

/// Send a pong message in response to a ping
pub async fn send_pong(ws: &mut WsStream, payload: Vec<u8>) -> Result<()> {
    ws.send(Message::Pong(payload.into())).await?;
    Ok(())
}

/// Send a text message
pub async fn send_text(ws: &mut WsStream, text: String) -> Result<()> {
    ws.send(Message::Text(text.into())).await?;
    Ok(())
}

/// Send a close frame
pub async fn send_close(ws: &mut WsStream) -> Result<()> {
    ws.send(Message::Close(None)).await?;
    Ok(())
}
