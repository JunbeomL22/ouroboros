# Binance WebSocket 샘플 프로젝트 구현 계획 (수정본)

## 디렉토리 구조

```
~/Projects/ouroboros/binance-websocket/webseocket/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── client.rs          # BinanceClient 구현
│   ├── connection.rs      # 연결 관리 및 재연결 로직
│   ├── subscription.rs    # 구독 상태 관리
│   ├── message.rs         # 메시지 타입 정의
│   └── error.rs           # 에러 타입 정의
└── tests/
    └── message_test.rs    # 메시지 파싱 테스트
```

## 의존성 (Cargo.toml)

```toml
[package]
name = "binance-websocket"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-native-roots"] }
futures-util = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
url = "2.5"
anyhow = "1.0"
thiserror = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rand = "0.8"
```

## 핵심 구현 사항

### 1. 엔드포인트 (api.md 기반)

**Single Stream**:
```
wss://stream.binance.com:9443/ws/<streamName>
```

**Combined Stream**:
```
wss://stream.binance.com:9443/stream?streams=<stream1>/<stream2>/<stream3>
```

### 2. Ping/Pong 처리 (api.md 기반)

| API 유형 | 서버 Ping 간격 | Pong 응답 제한 |
|----------|----------------|----------------|
| Spot | 20초 | 1분 이내 |
| Futures | 3분 | 10분 이내 |

- 서버가 Ping을 보내면 클라이언트가 동일 payload로 Pong 응답
- Unsolicited Pong 허용 (연결 유지용)

### 3. 메시지 타입 (message.rs)

```rust
// 구독 요청/응답
struct SubscribeRequest { method: String, params: Vec<String>, id: u64 }
struct SubscribeResponse { result: Option<serde_json::Value>, id: u64 }

// 에러 응답
struct ErrorResponse { code: i64, msg: String, id: Option<u64> }

// 마켓 데이터
struct TradeEvent { e: String, E: u64, s: String, p: String, q: String, ... }
struct BookTicker { u: u64, s: String, b: String, B: String, a: String, A: String }
struct DepthUpdate { e: String, E: u64, s: String, U: u64, u: u64, b: Vec<[String; 2]>, a: Vec<[String; 2]> }
```

### 4. 구독 관리 (subscription.rs)

```rust
struct SubscriptionManager {
    subscriptions: HashMap<u64, String>,  // id -> stream name
    pending_responses: HashMap<u64, oneshot::Sender<Result<()>>>,
    next_id: AtomicU64,
}
```

- 구독 ID 추적
- 응답 매칭
- 재연결 시 기존 구독 복원

### 5. 재연결 로직 (connection.rs)

```rust
struct ReconnectConfig {
    initial_delay: Duration,      // 1초
    max_delay: Duration,          // 60초
    max_retries: u32,             // 10회
    jitter_percent: f64,          // ±10%
}
```

지수 백오프 + 지터:
```
delay = min(initial * 2^attempt, max) * (1 ± jitter)
```

### 6. 에러 핸들링 (error.rs)

```rust
enum BinanceError {
    WebSocket(tungstenite::Error),
    Json(serde_json::Error),
    ConnectionClosed { code: u16, reason: String },
    MaxRetriesExceeded,
    SubscriptionFailed { id: u64, code: i64, msg: String },
}
```

Close 코드별 처리:
- 1000: 정상 종료, 재연결 불필요
- 1001, 1006, 1011: 백오프 후 재연결
- 1008: 정책 위반, 로그 후 백오프 재연결
- 1013: 서버 과부하, 지수 백오프 재연결

### 7. Graceful Shutdown

```rust
async fn shutdown(&mut self) {
    // 1. Close frame 전송 (code: 1000)
    // 2. 진행 중인 메시지 처리 완료 대기 (timeout 5초)
    // 3. 내부 상태 정리
}
```

### 8. 연결 제한 준수

| 제한 | 값 |
|------|-----|
| 연결 시도 | 300회/5분/IP |
| 메시지 전송 | 5개/초 (Spot) |
| 스트림/연결 | 최대 1024개 (권장 200개 이하) |
| Ping/Pong 레이트 | 5회/초 |

## 테스트 계획 (tests/message_test.rs)

```rust
#[test]
fn test_parse_trade_event() { ... }

#[test]
fn test_parse_book_ticker() { ... }

#[test]
fn test_parse_depth_update() { ... }

#[test]
fn test_parse_subscribe_response() { ... }

#[test]
fn test_parse_error_response() { ... }
```

## 예제 사용법 (main.rs)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = BinanceClient::new(ReconnectConfig::default());
    
    client.subscribe(&["btcusdt@trade", "ethusdt@bookTicker"]).await?;
    
    while let Some(event) = client.next().await {
        match event {
            MarketEvent::Trade(t) => println!("Trade: {} @ {}", t.symbol, t.price),
            MarketEvent::BookTicker(b) => println!("Bid: {}, Ask: {}", b.bid_price, b.ask_price),
            _ => {}
        }
    }
    
    Ok(())
}
```