//! Market data receiver for UDP and WebSocket connections
//!
//! This module provides the `MarketDataReceiver` struct for receiving market data
//! from multiple UDP sockets and WebSocket streams concurrently.

use async_tungstenite::{tokio::TokioAdapter, WebSocketStream};
use tokio::net::{TcpStream, UdpSocket};

/// Receiver for market data from multiple UDP and WebSocket connections.
///
/// This struct manages collections of UDP sockets and WebSocket streams
/// for receiving real-time market data from various sources.
pub struct MarketDataReceiver {
    /// UDP sockets for receiving multicast or unicast market data
    pub udp_sockets: Vec<UdpSocket>,
    /// WebSocket streams for receiving market data from exchanges
    pub websocket_streams: Vec<WebSocketStream<TokioAdapter<TcpStream>>>,
}
