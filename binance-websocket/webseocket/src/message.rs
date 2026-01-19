//! Message types for Binance WebSocket API

use serde::{Deserialize, Serialize};

/// Subscribe/Unsubscribe request message
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeRequest {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

impl SubscribeRequest {
    /// Create a new SUBSCRIBE request
    pub fn subscribe(streams: Vec<String>, id: u64) -> Self {
        Self {
            method: "SUBSCRIBE".to_string(),
            params: streams,
            id,
        }
    }

    /// Create a new UNSUBSCRIBE request
    pub fn unsubscribe(streams: Vec<String>, id: u64) -> Self {
        Self {
            method: "UNSUBSCRIBE".to_string(),
            params: streams,
            id,
        }
    }
}

/// Subscribe/Unsubscribe response message
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeResponse {
    pub result: Option<serde_json::Value>,
    pub id: u64,
}

/// Error response from Binance
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub code: i64,
    pub msg: String,
    pub id: Option<u64>,
}

/// Combined stream wrapper (for combined streams endpoint)
#[derive(Debug, Clone, Deserialize)]
pub struct CombinedStreamMessage<T> {
    pub stream: String,
    pub data: T,
}

/// Trade event from @trade stream
#[derive(Debug, Clone, Deserialize)]
pub struct TradeEvent {
    /// Event type (always "trade")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time (milliseconds)
    #[serde(rename = "E")]
    pub event_time: u64,

    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: u64,

    /// Price
    #[serde(rename = "p")]
    pub price: String,

    /// Quantity
    #[serde(rename = "q")]
    pub quantity: String,

    /// Trade time (milliseconds)
    #[serde(rename = "T")]
    pub trade_time: u64,

    /// Is the buyer the market maker?
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    /// Ignore
    #[serde(rename = "M")]
    pub ignore: bool,
}

/// Book ticker event from @bookTicker stream
#[derive(Debug, Clone, Deserialize)]
pub struct BookTicker {
    /// Order book updateId
    #[serde(rename = "u")]
    pub update_id: u64,

    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Best bid price
    #[serde(rename = "b")]
    pub bid_price: String,

    /// Best bid quantity
    #[serde(rename = "B")]
    pub bid_qty: String,

    /// Best ask price
    #[serde(rename = "a")]
    pub ask_price: String,

    /// Best ask quantity
    #[serde(rename = "A")]
    pub ask_qty: String,
}

/// Depth update event from @depth stream
#[derive(Debug, Clone, Deserialize)]
pub struct DepthUpdate {
    /// Event type (always "depthUpdate")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time (milliseconds)
    #[serde(rename = "E")]
    pub event_time: u64,

    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// First update ID in event
    #[serde(rename = "U")]
    pub first_update_id: u64,

    /// Final update ID in event
    #[serde(rename = "u")]
    pub final_update_id: u64,

    /// Bids to be updated [price, quantity]
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,

    /// Asks to be updated [price, quantity]
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
}

/// Aggregate trade event from @aggTrade stream
#[derive(Debug, Clone, Deserialize)]
pub struct AggTradeEvent {
    /// Event type (always "aggTrade")
    #[serde(rename = "e")]
    pub event_type: String,

    /// Event time (milliseconds)
    #[serde(rename = "E")]
    pub event_time: u64,

    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Aggregate trade ID
    #[serde(rename = "a")]
    pub agg_trade_id: u64,

    /// Price
    #[serde(rename = "p")]
    pub price: String,

    /// Quantity
    #[serde(rename = "q")]
    pub quantity: String,

    /// First trade ID
    #[serde(rename = "f")]
    pub first_trade_id: u64,

    /// Last trade ID
    #[serde(rename = "l")]
    pub last_trade_id: u64,

    /// Trade time (milliseconds)
    #[serde(rename = "T")]
    pub trade_time: u64,

    /// Is the buyer the market maker?
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    /// Ignore
    #[serde(rename = "M")]
    pub ignore: bool,
}

/// Market event enum for handling different stream types
#[derive(Debug, Clone)]
pub enum MarketEvent {
    Trade(TradeEvent),
    BookTicker(BookTicker),
    DepthUpdate(DepthUpdate),
    AggTrade(AggTradeEvent),
    SubscribeResponse(SubscribeResponse),
    Error(ErrorResponse),
    Unknown(String),
}

impl MarketEvent {
    /// Parse a raw JSON message into a MarketEvent
    pub fn parse(text: &str) -> Self {
        // Parse JSON once
        let value = match serde_json::from_str::<serde_json::Value>(text) {
            Ok(v) => v,
            Err(_) => return MarketEvent::Unknown(text.to_string()),
        };

        // Try to parse as error response (error has 'code' and 'msg' fields)
        // Check this BEFORE subscribe response since both may have 'id'
        if value.get("code").is_some() && value.get("msg").is_some() {
            if let Ok(err) = serde_json::from_value::<ErrorResponse>(value.clone()) {
                return MarketEvent::Error(err);
            }
        }

        // Try to parse as subscribe response (has 'result' and 'id', no 'code'/'msg')
        if value.get("result").is_some() && value.get("id").is_some() {
            if let Ok(resp) = serde_json::from_value::<SubscribeResponse>(value.clone()) {
                return MarketEvent::SubscribeResponse(resp);
            }
        }

        // Try to detect event type from JSON
        {
            let value = value;
            // Handle combined stream format
            let data = if value.get("stream").is_some() {
                value.get("data").cloned().unwrap_or_else(|| value.clone())
            } else {
                value
            };

            if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
                match event_type {
                    "trade" => {
                        if let Ok(trade) = serde_json::from_value::<TradeEvent>(data.clone()) {
                            return MarketEvent::Trade(trade);
                        }
                    }
                    "depthUpdate" => {
                        if let Ok(depth) = serde_json::from_value::<DepthUpdate>(data.clone()) {
                            return MarketEvent::DepthUpdate(depth);
                        }
                    }
                    "aggTrade" => {
                        if let Ok(agg) = serde_json::from_value::<AggTradeEvent>(data.clone()) {
                            return MarketEvent::AggTrade(agg);
                        }
                    }
                    _ => {}
                }
            }

            // Try bookTicker (no event type field)
            if data.get("u").is_some() && data.get("b").is_some() && data.get("a").is_some() {
                if let Ok(ticker) = serde_json::from_value::<BookTicker>(data) {
                    return MarketEvent::BookTicker(ticker);
                }
            }
        }

        MarketEvent::Unknown(text.to_string())
    }
}
