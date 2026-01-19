//! Tests for message parsing

use binance_websocket::MarketEvent;

#[test]
fn test_parse_trade_event() {
    let json = r#"{
        "e": "trade",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "t": 12345,
        "p": "42000.00",
        "q": "0.001",
        "T": 1672515782136,
        "m": true,
        "M": true
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::Trade(trade) => {
            assert_eq!(trade.event_type, "trade");
            assert_eq!(trade.symbol, "BTCUSDT");
            assert_eq!(trade.price, "42000.00");
            assert_eq!(trade.quantity, "0.001");
            assert_eq!(trade.trade_id, 12345);
            assert!(trade.is_buyer_maker);
        }
        _ => panic!("Expected Trade event"),
    }
}

#[test]
fn test_parse_book_ticker() {
    let json = r#"{
        "u": 400900217,
        "s": "ETHUSDT",
        "b": "2500.00",
        "B": "10.5",
        "a": "2500.10",
        "A": "8.2"
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::BookTicker(ticker) => {
            assert_eq!(ticker.symbol, "ETHUSDT");
            assert_eq!(ticker.bid_price, "2500.00");
            assert_eq!(ticker.bid_qty, "10.5");
            assert_eq!(ticker.ask_price, "2500.10");
            assert_eq!(ticker.ask_qty, "8.2");
            assert_eq!(ticker.update_id, 400900217);
        }
        _ => panic!("Expected BookTicker event"),
    }
}

#[test]
fn test_parse_depth_update() {
    let json = r#"{
        "e": "depthUpdate",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "U": 157,
        "u": 160,
        "b": [["42000.00", "1.5"], ["41999.00", "2.0"]],
        "a": [["42001.00", "0.8"], ["42002.00", "1.2"]]
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::DepthUpdate(depth) => {
            assert_eq!(depth.event_type, "depthUpdate");
            assert_eq!(depth.symbol, "BTCUSDT");
            assert_eq!(depth.first_update_id, 157);
            assert_eq!(depth.final_update_id, 160);
            assert_eq!(depth.bids.len(), 2);
            assert_eq!(depth.asks.len(), 2);
            assert_eq!(depth.bids[0], ["42000.00", "1.5"]);
            assert_eq!(depth.asks[0], ["42001.00", "0.8"]);
        }
        _ => panic!("Expected DepthUpdate event"),
    }
}

#[test]
fn test_parse_agg_trade() {
    let json = r#"{
        "e": "aggTrade",
        "E": 1672515782136,
        "s": "BTCUSDT",
        "a": 12345,
        "p": "42000.00",
        "q": "0.5",
        "f": 100,
        "l": 105,
        "T": 1672515782136,
        "m": false,
        "M": true
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::AggTrade(agg) => {
            assert_eq!(agg.event_type, "aggTrade");
            assert_eq!(agg.symbol, "BTCUSDT");
            assert_eq!(agg.agg_trade_id, 12345);
            assert_eq!(agg.price, "42000.00");
            assert_eq!(agg.quantity, "0.5");
            assert_eq!(agg.first_trade_id, 100);
            assert_eq!(agg.last_trade_id, 105);
            assert!(!agg.is_buyer_maker);
        }
        _ => panic!("Expected AggTrade event"),
    }
}

#[test]
fn test_parse_subscribe_response() {
    let json = r#"{
        "result": null,
        "id": 1
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::SubscribeResponse(resp) => {
            assert_eq!(resp.id, 1);
            assert!(resp.result.is_none());
        }
        _ => panic!("Expected SubscribeResponse event"),
    }
}

#[test]
fn test_parse_error_response() {
    let json = r#"{
        "code": -1102,
        "msg": "Invalid request",
        "id": 2
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::Error(err) => {
            assert_eq!(err.code, -1102);
            assert_eq!(err.msg, "Invalid request");
            assert_eq!(err.id, Some(2));
        }
        other => panic!("Expected Error event, got: {:?}", other),
    }
}

#[test]
fn test_parse_combined_stream_trade() {
    let json = r#"{
        "stream": "btcusdt@trade",
        "data": {
            "e": "trade",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "t": 12345,
            "p": "42000.00",
            "q": "0.001",
            "T": 1672515782136,
            "m": true,
            "M": true
        }
    }"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::Trade(trade) => {
            assert_eq!(trade.symbol, "BTCUSDT");
            assert_eq!(trade.price, "42000.00");
        }
        _ => panic!("Expected Trade event from combined stream"),
    }
}

#[test]
fn test_parse_unknown_message() {
    let json = r#"{"foo": "bar"}"#;

    let event = MarketEvent::parse(json);

    match event {
        MarketEvent::Unknown(raw) => {
            assert!(raw.contains("foo"));
        }
        _ => panic!("Expected Unknown event"),
    }
}
