//! Binance WebSocket Client Library
//!
//! A Rust library for connecting to Binance WebSocket streams.
//!
//! # Example
//!
//! ```no_run
//! use binance_websocket::{BinanceClient, ReconnectConfig, MarketEvent};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = BinanceClient::new("btcusdt@trade", ReconnectConfig::default());
//!     client.connect().await?;
//!
//!     while let Some(event) = client.next().await {
//!         match event {
//!             MarketEvent::Trade(trade) => {
//!                 println!("Trade: {} @ {}", trade.symbol, trade.price);
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod connection;
pub mod error;
pub mod message;
pub mod subscription;

// Re-exports for convenience
pub use client::{BinanceClient, BINANCE_WS_SPOT, BINANCE_WS_SPOT_COMBINED, BINANCE_WS_FUTURES};
pub use connection::ReconnectConfig;
pub use error::{BinanceError, Result};
pub use message::{
    AggTradeEvent, BookTicker, DepthUpdate, MarketEvent, SubscribeRequest, SubscribeResponse,
    TradeEvent,
};
pub use subscription::SubscriptionManager;
