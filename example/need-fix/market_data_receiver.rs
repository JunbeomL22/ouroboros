//! Market data receiver for UDP and WebSocket connections
//!
//! This module provides the `MarketDataReceiver` struct for receiving market data
//! from multiple UDP sockets and WebSocket streams concurrently.

use async_std::net::UdpSocket;
use async_tungstenite::{tungstenite::Message, WebSocketStream};
use futures::stream::StreamExt;

/// Result of polling UDP sockets.
#[derive(Debug)]
pub enum UdpPollResult {
    /// Data received from a socket (socket index, data, source address)
    Data(usize, Vec<u8>, std::net::SocketAddr),
    /// No sockets ready
    NotReady,
    /// No sockets configured
    NoSockets,
}

/// Result of polling WebSocket streams.
#[derive(Debug)]
pub enum WebSocketPollResult {
    /// Message received from a stream (stream index, message)
    Message(usize, Message),
    /// Stream closed (stream index)
    Closed(usize),
    /// Error on a stream (stream index)
    Error(usize, async_tungstenite::tungstenite::Error),
    /// No streams ready
    NotReady,
    /// No streams configured
    NoStreams,
}

/// Receiver for market data from multiple UDP and WebSocket connections.
///
/// This struct manages collections of UDP sockets and WebSocket streams
/// for receiving real-time market data from various sources.
pub struct MarketDataReceiver {
    /// UDP sockets for receiving multicast or unicast market data (async-std)
    pub udp_sockets: Vec<UdpSocket>,
    /// WebSocket streams for receiving market data from exchanges (async-tungstenite)
    pub websocket_streams: Vec<WebSocketStream<async_std::net::TcpStream>>,
}

/// Default UDP buffer size (64KB)
const DEFAULT_UDP_BUFFER_SIZE: usize = 65536;

impl MarketDataReceiver {
    /// Creates a new `MarketDataReceiver` with empty socket collections.
    #[must_use]
    pub fn new() -> Self {
        Self {
            udp_sockets: Vec::new(),
            websocket_streams: Vec::new(),
        }
    }

    /// Adds a UDP socket to the receiver.
    ///
    /// # Arguments
    ///
    /// * `socket` - The UDP socket to add for receiving market data
    pub fn add_udp_socket(&mut self, socket: UdpSocket) {
        self.udp_sockets.push(socket);
    }

    /// Adds a WebSocket stream to the receiver.
    ///
    /// # Arguments
    ///
    /// * `stream` - The WebSocket stream to add for receiving market data
    pub fn add_websocket(&mut self, stream: WebSocketStream<async_std::net::TcpStream>) {
        self.websocket_streams.push(stream);
    }

    /// Removes a UDP socket at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the UDP socket to remove
    ///
    /// # Returns
    ///
    /// The removed UDP socket if the index was valid, `None` otherwise
    pub fn remove_udp_socket(&mut self, index: usize) -> Option<UdpSocket> {
        if index < self.udp_sockets.len() {
            Some(self.udp_sockets.remove(index))
        } else {
            None
        }
    }

    /// Removes a WebSocket stream at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the WebSocket stream to remove
    ///
    /// # Returns
    ///
    /// The removed WebSocket stream if the index was valid, `None` otherwise
    pub fn remove_websocket(
        &mut self,
        index: usize,
    ) -> Option<WebSocketStream<async_std::net::TcpStream>> {
        if index < self.websocket_streams.len() {
            Some(self.websocket_streams.remove(index))
        } else {
            None
        }
    }

    /// Polls all UDP sockets for incoming data using epoll-style polling.
    ///
    /// This method iterates through all UDP sockets and attempts to receive data
    /// from each one without blocking. Returns immediately with the first socket
    /// that has data available.
    ///
    /// # Returns
    ///
    /// - `UdpPollResult::Data(index, data, addr)` - Data received from socket at index
    /// - `UdpPollResult::NotReady` - No sockets have data available
    /// - `UdpPollResult::NoSockets` - No UDP sockets configured
    pub async fn poll_udp_sockets(&mut self) -> UdpPollResult {
        if self.udp_sockets.is_empty() {
            return UdpPollResult::NoSockets;
        }

        // Use select_all-style polling to check all sockets concurrently
        let futures: Vec<_> = self
            .udp_sockets
            .iter()
            .enumerate()
            .map(|(idx, socket)| {
                Box::pin(async move {
                    let mut buf = vec![0u8; DEFAULT_UDP_BUFFER_SIZE];
                    match socket.recv_from(&mut buf).await {
                        Ok((len, addr)) => {
                            buf.truncate(len);
                            Some((idx, buf, addr))
                        }
                        Err(_) => None,
                    }
                })
            })
            .collect();

        if futures.is_empty() {
            return UdpPollResult::NotReady;
        }

        // Race all socket recv operations
        let (result, _index, _remaining) = futures::future::select_all(futures).await;

        match result {
            Some((idx, data, addr)) => UdpPollResult::Data(idx, data, addr),
            None => UdpPollResult::NotReady,
        }
    }

    /// Polls all WebSocket streams for incoming messages using epoll-style polling.
    ///
    /// This method iterates through all WebSocket streams and attempts to receive
    /// a message from any stream that has data available. Returns immediately with
    /// the first message received.
    ///
    /// # Returns
    ///
    /// - `WebSocketPollResult::Message(index, msg)` - Message received from stream at index
    /// - `WebSocketPollResult::Closed(index)` - Stream at index was closed
    /// - `WebSocketPollResult::Error(index, err)` - Error on stream at index
    /// - `WebSocketPollResult::NotReady` - No streams have messages available
    /// - `WebSocketPollResult::NoStreams` - No WebSocket streams configured
    pub async fn poll_websockets(&mut self) -> WebSocketPollResult {
        if self.websocket_streams.is_empty() {
            return WebSocketPollResult::NoStreams;
        }

        // Create futures for polling each WebSocket stream
        let futures: Vec<_> = self
            .websocket_streams
            .iter_mut()
            .enumerate()
            .map(|(idx, stream)| {
                Box::pin(async move {
                    match stream.next().await {
                        Some(Ok(msg)) => (idx, Some(Ok(msg))),
                        Some(Err(e)) => (idx, Some(Err(e))),
                        None => (idx, None),
                    }
                })
            })
            .collect();

        if futures.is_empty() {
            return WebSocketPollResult::NotReady;
        }

        // Race all WebSocket stream operations
        let ((idx, result), _index, _remaining) = futures::future::select_all(futures).await;

        match result {
            Some(Ok(msg)) => WebSocketPollResult::Message(idx, msg),
            Some(Err(e)) => WebSocketPollResult::Error(idx, e),
            None => WebSocketPollResult::Closed(idx),
        }
    }
}

impl Default for MarketDataReceiver {
    fn default() -> Self {
        Self::new()
    }
}
