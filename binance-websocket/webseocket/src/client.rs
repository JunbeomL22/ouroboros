//! Binance WebSocket client implementation

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};

use crate::connection::{ConnectionManager, ReconnectConfig, WsStream};
use crate::error::{BinanceError, Result};
use crate::message::{MarketEvent, SubscribeRequest};
use crate::subscription::SubscriptionManager;

/// Binance WebSocket base URLs
pub const BINANCE_WS_SPOT: &str = "wss://stream.binance.com:9443";
pub const BINANCE_WS_SPOT_COMBINED: &str = "wss://stream.binance.com:9443/stream";
pub const BINANCE_WS_FUTURES: &str = "wss://fstream.binance.com";

/// Binance WebSocket client
pub struct BinanceClient {
    /// Connection manager
    conn_manager: Arc<Mutex<ConnectionManager>>,

    /// Subscription manager
    sub_manager: Arc<RwLock<SubscriptionManager>>,

    /// Event sender
    event_tx: mpsc::Sender<MarketEvent>,

    /// Event receiver (for consuming events)
    event_rx: Arc<Mutex<mpsc::Receiver<MarketEvent>>>,

    /// Write half of the WebSocket connection
    write: Arc<Mutex<Option<SplitSink<WsStream, Message>>>>,

    /// Flag to indicate if the client is running
    is_running: Arc<RwLock<bool>>,
}

impl BinanceClient {
    /// Create a new Binance WebSocket client for single stream
    pub fn new(stream: &str, config: ReconnectConfig) -> Self {
        let url = format!("{}/ws/{}", BINANCE_WS_SPOT, stream);
        Self::with_url(url, config)
    }

    /// Create a new Binance WebSocket client for combined streams
    pub fn combined(streams: &[&str], config: ReconnectConfig) -> Self {
        let streams_param = streams.join("/");
        let url = format!("{}?streams={}", BINANCE_WS_SPOT_COMBINED, streams_param);
        Self::with_url(url, config)
    }

    /// Create a new Binance WebSocket client with a custom URL
    pub fn with_url(url: String, config: ReconnectConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        Self {
            conn_manager: Arc::new(Mutex::new(ConnectionManager::new(url, config))),
            sub_manager: Arc::new(RwLock::new(SubscriptionManager::new())),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            write: Arc::new(Mutex::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Connect to the WebSocket server and start processing messages
    pub async fn connect(&self) -> Result<()> {
        let mut conn = self.conn_manager.lock().await;
        let ws = conn.connect_with_retry().await?;

        let (write, read) = ws.split();
        *self.write.lock().await = Some(write);
        *self.is_running.write().await = true;

        // Start the message processing loop
        self.spawn_read_loop(read);

        Ok(())
    }

    /// Spawn the read loop as a background task
    fn spawn_read_loop(&self, mut read: SplitStream<WsStream>) {
        let event_tx = self.event_tx.clone();
        let write = self.write.clone();
        let is_running = self.is_running.clone();
        let sub_manager = self.sub_manager.clone();
        let conn_manager = self.conn_manager.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                match read.next().await {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                let event = MarketEvent::parse(&text);

                                // Handle subscribe responses
                                if let MarketEvent::SubscribeResponse(ref resp) = event {
                                    sub_manager.write().await.confirm_subscription(resp.id);
                                }

                                // Handle error responses
                                if let MarketEvent::Error(ref err) = event {
                                    if let Some(id) = err.id {
                                        sub_manager.write().await.fail_subscription(
                                            id,
                                            BinanceError::SubscriptionFailed {
                                                id,
                                                code: err.code,
                                                msg: err.msg.clone(),
                                            },
                                        );
                                    }
                                }

                                if event_tx.send(event).await.is_err() {
                                    tracing::error!("Failed to send event to channel");
                                    break;
                                }
                            }
                            Message::Ping(payload) => {
                                // Respond to ping with pong
                                if let Some(ref mut w) = *write.lock().await {
                                    if let Err(e) = w.send(Message::Pong(payload.into())).await {
                                        tracing::error!("Failed to send pong: {}", e);
                                    }
                                }
                            }
                            Message::Pong(_) => {
                                // Pong received, connection is healthy
                                conn_manager.lock().await.reset_attempts();
                            }
                            Message::Close(frame) => {
                                let (code, reason) = frame
                                    .map(|f| (f.code.into(), f.reason.to_string()))
                                    .unwrap_or((1006, "Abnormal closure".to_string()));

                                tracing::warn!(
                                    "WebSocket closed: code={}, reason={}",
                                    code,
                                    reason
                                );

                                *is_running.write().await = false;
                                break;
                            }
                            Message::Binary(_) => {
                                // Binary messages are not typically used
                                tracing::debug!("Received binary message");
                            }
                            Message::Frame(_) => {
                                // Raw frame, usually not needed
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        *is_running.write().await = false;
                        break;
                    }
                    None => {
                        tracing::info!("WebSocket stream ended");
                        *is_running.write().await = false;
                        break;
                    }
                }
            }
        });
    }

    /// Subscribe to additional streams (after connection is established)
    pub async fn subscribe(&self, streams: &[&str]) -> Result<()> {
        let mut sub_manager = self.sub_manager.write().await;
        let id = sub_manager.next_id();

        let stream_names: Vec<String> = streams.iter().map(|s| s.to_string()).collect();
        let request = SubscribeRequest::subscribe(stream_names.clone(), id);
        let json = serde_json::to_string(&request)?;

        // Register subscriptions
        for stream in &stream_names {
            sub_manager.register_subscription(id, stream.clone());
        }

        // Send subscribe request
        if let Some(ref mut write) = *self.write.lock().await {
            write.send(Message::Text(json.into())).await?;
        }

        Ok(())
    }

    /// Unsubscribe from streams
    pub async fn unsubscribe(&self, streams: &[&str]) -> Result<()> {
        let mut sub_manager = self.sub_manager.write().await;
        let id = sub_manager.next_id();

        let stream_names: Vec<String> = streams.iter().map(|s| s.to_string()).collect();
        let request = SubscribeRequest::unsubscribe(stream_names.clone(), id);
        let json = serde_json::to_string(&request)?;

        // Remove subscriptions from manager
        for stream in &stream_names {
            sub_manager.remove_subscription(stream);
        }

        // Send unsubscribe request
        if let Some(ref mut write) = *self.write.lock().await {
            write.send(Message::Text(json.into())).await?;
        }

        Ok(())
    }

    /// Get the next event from the stream
    pub async fn next(&self) -> Option<MarketEvent> {
        self.event_rx.lock().await.recv().await
    }

    /// Check if the client is currently connected
    pub async fn is_connected(&self) -> bool {
        *self.is_running.read().await
    }

    /// Gracefully shutdown the connection
    pub async fn shutdown(&self) -> Result<()> {
        *self.is_running.write().await = false;

        // Send close frame
        if let Some(ref mut write) = *self.write.lock().await {
            let _ = write.send(Message::Close(None)).await;
        }

        // Clear pending subscriptions
        self.sub_manager.write().await.clear_pending();

        tracing::info!("WebSocket client shutdown complete");
        Ok(())
    }

    /// Get the number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.sub_manager.read().await.subscription_count()
    }
}
