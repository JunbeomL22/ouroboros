//! Binance WebSocket Client Example
//!
//! This example demonstrates how to connect to Binance WebSocket streams
//! and receive real-time market data.

use anyhow::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use binance_websocket::{BinanceClient, MarketEvent, ReconnectConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("binance_websocket=info".parse()?))
        .init();

    tracing::info!("Starting Binance WebSocket client");

    // Create a client for combined streams
    let client = BinanceClient::combined(
        &["btcusdt@trade", "ethusdt@bookTicker"],
        ReconnectConfig::default(),
    );

    // Connect to the WebSocket server
    client.connect().await?;
    tracing::info!("Connected to Binance WebSocket");

    // Process incoming events
    let mut trade_count = 0;
    let mut ticker_count = 0;

    while let Some(event) = client.next().await {
        match event {
            MarketEvent::Trade(trade) => {
                trade_count += 1;
                println!(
                    "[TRADE] {} | Price: {} | Qty: {} | Time: {} | Maker: {}",
                    trade.symbol,
                    trade.price,
                    trade.quantity,
                    trade.trade_time,
                    if trade.is_buyer_maker { "Sell" } else { "Buy" }
                );

                // Stop after 10 trades for demo purposes
                if trade_count >= 10 {
                    tracing::info!("Received 10 trades, shutting down...");
                    break;
                }
            }
            MarketEvent::BookTicker(ticker) => {
                ticker_count += 1;
                println!(
                    "[TICKER] {} | Bid: {} ({}) | Ask: {} ({})",
                    ticker.symbol,
                    ticker.bid_price,
                    ticker.bid_qty,
                    ticker.ask_price,
                    ticker.ask_qty
                );
            }
            MarketEvent::DepthUpdate(depth) => {
                println!(
                    "[DEPTH] {} | Bids: {} | Asks: {}",
                    depth.symbol,
                    depth.bids.len(),
                    depth.asks.len()
                );
            }
            MarketEvent::AggTrade(agg) => {
                println!(
                    "[AGGTRADE] {} | Price: {} | Qty: {}",
                    agg.symbol, agg.price, agg.quantity
                );
            }
            MarketEvent::SubscribeResponse(resp) => {
                tracing::info!("Subscription confirmed: id={}", resp.id);
            }
            MarketEvent::Error(err) => {
                tracing::error!("Error: code={}, msg={}", err.code, err.msg);
            }
            MarketEvent::Unknown(raw) => {
                tracing::debug!("Unknown message: {}", raw);
            }
        }
    }

    // Graceful shutdown
    client.shutdown().await?;
    tracing::info!(
        "Shutdown complete. Received {} trades, {} tickers",
        trade_count,
        ticker_count
    );

    Ok(())
}
