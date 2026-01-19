# Rust WebSocket 라이브러리: 암호화폐 시장 연결을 위한 베스트 프랙티스

## 목차
1. [개요](#1-개요)
2. [주요 WebSocket 라이브러리 비교](#2-주요-websocket-라이브러리-비교)
3. [암호화폐 거래소 전용 라이브러리](#3-암호화폐-거래소-전용-라이브러리)
4. [TLS 보안 구현](#4-tls-보안-구현)
5. [연결 관리 베스트 프랙티스](#5-연결-관리-베스트-프랙티스)
6. [메시지 파싱 및 직렬화](#6-메시지-파싱-및-직렬화)
7. [동시성 및 Backpressure 관리](#7-동시성-및-backpressure-관리)
8. [권장 아키텍처](#8-권장-아키텍처)
9. [Binance WebSocket 연결 관리 상세](#9-binance-websocket-연결-관리-상세)
10. [JSON 파싱 최적화](#10-json-파싱-최적화)
11. [에러 핸들링](#11-에러-핸들링)
12. [실제 코드 예시](#12-실제-코드-예시)
13. [참고 자료](#13-참고-자료)

---

## 1. 개요

Rust는 메모리 안전성과 고성능을 동시에 제공하여 실시간 암호화폐 시장 데이터 처리에 최적화된 언어이다. 가비지 컬렉션이 없어 99번째 백분위 지연시간이 일관되게 낮으며, 제로코스트 추상화로 고수준 코드 패턴에서도 숨겨진 성능 페널티가 없다.

### Rust WebSocket의 주요 장점
- **메모리 안전성**: 컴파일 타임에 메모리 오류 방지
- **성능**: 가비지 컬렉션 없음, 제로코스트 추상화
- **동시성**: Tokio 기반 async/await 생태계로 단일 스레드에서 수천 개의 동시 연결 처리 가능
- **타입 안전성**: 런타임 리플렉션 없이 강타입 직렬화/역직렬화

---

## 2. 주요 WebSocket 라이브러리 비교

### 2.1 성능 벤치마크 (ws-bench 기준)

| 라이브러리 | 평균 시간 (ms) | 특징 |
|------------|----------------|------|
| **wtx** | 6,350.31 | 가장 빠름, SIMD 최적화 |
| **tokio-tungstenite** | 7,602.94 | 가장 널리 사용됨 |
| **uWebSockets** | 8,393.94 | C++ 기반 |
| **fastwebsockets** | 10,140.58 | Deno 팀 개발 |
| **gorilla/websockets** | 10,900.23 | Go 언어 |

*테스트 환경: i5-1135G7, 256GB SSD, 32GB RAM*

### 2.2 라이브러리별 상세 분석

#### tokio-tungstenite (권장)
- **GitHub**: [snapview/tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
- **최신 버전**: 0.27.0 (2025-06-13 릴리스)
- **다운로드**: 월 137,000+ (lib.rs 기준)
- **특징**:
  - Tokio 런타임과 완벽 통합
  - tungstenite의 비동기 래퍼
  - 최근 버전(>0.26.2)에서 성능 대폭 개선, fastwebsockets와 동등 수준
  - native-tls, rustls 모두 지원

```toml
[dependencies]
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-native-roots"] }
```

#### tokio-websockets (SIMD 최적화)
- **GitHub**: [Gelbpunkt/tokio-websockets](https://github.com/Gelbpunkt/tokio-websockets)
- **특징**:
  - AVX2/SSE2 SIMD 가속 마스킹 및 UTF-8 검증
  - 불필요한 UTF-8 중복 검증 제거
  - 중복 경계 검사 없음 (unsafe 코드 많음)
  - 높은 성능 튜닝

```toml
[dependencies]
# SIMD 활성화 클라이언트
tokio-websockets = { version = "0.12", features = ["client", "simd", "rustls-webpki-roots"] }
```

#### wtx (최고 성능)
- **Crates.io**: [wtx](https://crates.io/crates/wtx)
- **벤치마크**: [bencher.dev/perf/wtx](https://bencher.dev/perf/wtx)
- **특징**:
  - 모든 벤치마크에서 최고 성능
  - 수동 벡터화로 SIMD 최적화
  - 힙 할당 최소화, 스택 메모리 우선 사용
  - 인스턴스 생성 시 한 번만 할당

```toml
[dependencies]
wtx = "0.36"
```

#### fastwebsockets (Deno 팀)
- **GitHub**: [denoland/fastwebsockets](https://github.com/denoland/fastwebsockets)
- **특징**:
  - LLVM libfuzzer로 퍼징 테스트 완료
  - Autobahn|TestSuite 통과
  - FIN 설정된 원시 프레임 제공 (tungstenite는 연결된 메시지 제공)
  - 최소한의 구현으로 빠른 속도

**⚠️ 주의사항**: 일부 벤치마크에서 "unsound and not thread-safe, non-strict spec compliance"로 특성화됨. 테스트 시나리오에 따라 성능 편차가 큼.

```toml
[dependencies]
fastwebsockets = "0.8"
```

#### ezsockets (높은 추상화)
- **GitHub**: [gbaranski/ezsockets](https://github.com/gbaranski/ezsockets)
- **특징**:
  - 자동 재연결 내장
  - 자동 Ping/Pong 처리
  - 선언적 이벤트 기반 프로그래밍
  - Axum, Tungstenite 백엔드 지원
  - WASM 클라이언트 지원

```toml
[dependencies]
ezsockets = "0.6"
```

### 2.3 라이브러리 선택 가이드

| 요구사항 | 권장 라이브러리 |
|----------|-----------------|
| 일반적인 프로덕션 사용 | tokio-tungstenite |
| 최고 성능 (HFT) | wtx |
| 빠른 개발, 자동 재연결 필요 | ezsockets |
| 최소한의 의존성 | fastwebsockets |
| 멀티 거래소 통합 | barter-data |

---

## 3. 암호화폐 거래소 전용 라이브러리

### 3.1 공식 Binance Rust SDK

- **GitHub**: [binance/binance-connector-rust](https://github.com/binance/binance-connector-rust)
- **설명**: Binance API의 공식 자동 생성 Rust SDK

```toml
[dependencies]
binance-sdk = { version = "1.0.0", features = ["derivatives_trading_usds_futures", "spot"] }
```

### 3.2 멀티 거래소 라이브러리

#### barter-data (추천)
- **문서**: [docs.rs/barter-data](https://docs.rs/barter-data)
- **GitHub**: [barter-rs/barter-rs](https://github.com/barter-rs/barter-rs)
- **지원 거래소**:
  - **Binance** (Spot, Futures)
  - **Bybit** (Spot: `BybitSpot`, Perpetuals: `BybitPerpetualsUsd`)
  - **Gate.io** (Spot: `GateioSpot`, Perpetuals: `GateioPerpetualsBtc`/`GateioPerpetualsUsd`, Options: `GateioOptions`)
  - **OKX**
  - **Coinbase**
  - **Bitmex**
- **특징**:
  - 정규화된 데이터 모델
  - tick-by-tick 실시간 데이터
  - StreamBuilder로 간편한 설정
  - MIT 라이선스
  - **⚠️ 주의**: 교육 및 연구 목적 전용, 프로덕션/라이브 트레이딩용 아님

```rust
use barter_data::builder::StreamBuilder;
use barter_data::exchange::binance::spot::BinanceSpot;

let streams = StreamBuilder::new()
    .subscribe(BinanceSpot::default().trades("btcusdt"))
    .build()
    .await?;
```

#### crypto-ws-client
- **문서**: [docs.rs/crypto-ws-client](https://docs.rs/crypto-ws-client)
- **지원 거래소**: Binance, Coinbase Pro, Kraken (Spot/Futures)
- **특징**: 다양한 거래소의 WebSocket 클라이언트 통합

```toml
[dependencies]
crypto-ws-client = "4.0"
```

#### crypto-botters
- **Lib.rs**: [crypto-botters](https://lib.rs/crates/crypto-botters)
- **특징**:
  - `Client::websocket()` 메서드로 연결 관리
  - 메시지 전송, 재연결 요청, 연결 종료 제어
  - `DeserializeOwned` 구현 타입으로 자동 역직렬화

### 3.3 단일 거래소 라이브러리

#### binance-rs (비공식)
- **GitHub**: [ccxt/binance-rs](https://github.com/wisespace-io/binance-rs)
- **특징**: Binance Spot/Futures API 지원, WebSocket 스트림 포함

#### binance-futures
- **Lib.rs**: [binance-futures](https://lib.rs/crates/binance-futures)
- **특징**:
  - 멀티플렉싱 매니저/컨트롤러 아키텍처
  - 단일 연결에서 여러 토픽 동시 스트리밍
  - 스레드 안전 채널로 메시지 라우팅
  - 강타입 enum으로 제로코스트 추상화

#### binance-async-rs
- **GitHub**: [dovahcrow/binance-async-rs](https://github.com/dovahcrow/binance-async-rs)
- **특징**: Async/Await, 인체공학적 설계

---

## 4. TLS 보안 구현

### 4.1 native-tls vs rustls 비교

| 항목 | native-tls | rustls |
|------|------------|--------|
| **보안** | 플랫폼 의존 (OpenSSL 등) | 메모리 안전 설계, CNCF 감사 통과 |
| **성능** | 플랫폼별 차이 | 일반적으로 더 빠름 |
| **이식성** | 시스템 의존성 필요 | 완전 이식 가능 |
| **인증서 관리** | 시스템 저장소 자동 사용 | 수동 관리 필요 |
| **암호화 백엔드** | 플랫폼 네이티브 | aws-lc-rs 또는 ring |

### 4.2 aws-lc-rs vs ring 암호화 백엔드 성능 비교

rustls는 두 가지 암호화 백엔드를 지원한다:

| 항목 | aws-lc-rs | ring |
|------|-----------|------|
| **대량 전송** | 최대 67% 더 빠름 | 기준 |
| **TLS 1.3 핸드셰이크** | 최대 16% 느림 | 기준 |
| **권장 상황** | 데이터 전송량 많은 경우 | 핸드셰이크 빈번한 경우 |
| **기본값** | rustls 0.23+ 기본값 | 피처 플래그로 선택 |

**주요 발견 사항** ([rustls-bench-results](https://github.com/aochagavia/rustls-bench-results)):
- aws-lc-rs는 GCC와 Clang 사이에 유의미한 성능 차이 없음
- ring은 GCC로 컴파일 시 서버 측 핸드셰이크 지연이 최대 21% 증가 (Clang 권장)
- rustls 0.23.36 이상에서 aws-lc-rs 백엔드 사용 시 포스트 퀀텀 키 교환(X25519MLKEM768) 기본 지원

### 4.3 메모리 할당자 최적화

**jemalloc 사용 시 성능 향상** ([jemalloc 최적화](https://leapcell.medium.com/optimizing-rust-performance-with-jemalloc-c18057532194)):

| 할당자 | 데이터 전송 처리량 향상 |
|--------|-------------------------|
| glibc malloc (기본) | 기준 |
| jemalloc | 35% ~ 136% 향상 (암호화 백엔드 및 암호 스위트에 따라 다름) |

**메모리 사용량 비교**:
- rustls 세션: 피크 시 약 13KiB
- OpenSSL 세션: 피크 시 약 69KiB
- C10K 시나리오: rustls 132MiB vs OpenSSL 688MiB

```rust
// jemalloc 활성화
use tikv_jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
```

```toml
[dependencies]
tikv-jemallocator = "0.5"
```

### 4.4 권장 사항

**프로덕션 환경에서는 rustls + aws-lc-rs + jemalloc 권장**:
- 메모리 안전성 보장
- 일관된 크로스 플랫폼 동작
- 구식 암호화 기본 비활성화
- 2020년 CNCF 보안 감사 통과
- 대량 데이터 전송에서 최고 성능

```toml
# tokio-tungstenite with rustls (aws-lc-rs 기본 사용)
[dependencies]
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-native-roots"] }
tikv-jemallocator = "0.5"

# 또는 webpki roots 사용
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-webpki-roots"] }
```

### 4.5 주의사항
- rustls는 시스템 인증서 저장소를 자동으로 사용하지 않음
- `native-roots` 또는 `webpki-roots` 피처 플래그로 루트 인증서 설정 필요

---

## 5. 연결 관리 베스트 프랙티스

### 5.1 자동 재연결 (Exponential Backoff)

```rust
use reconnecting_websocket::ReconnectingWebSocket;
use exponential_backoff::Backoff;

// reconnecting-websocket 크레이트 사용
let ws = ReconnectingWebSocket::new(url, backoff_config).await?;
```

#### 권장 백오프 전략
- **초기 지연**: 100ms
- **최대 지연**: 30초
- **지터(Jitter) 추가**: 서버 과부하 시 재연결 분산

### 5.2 Heartbeat/Ping-Pong 구현

```rust
// 권장 설정
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20); // 20-30초 권장
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

async fn heartbeat_loop(ws: &mut WebSocket) {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        interval.tick().await;
        ws.send(Message::Ping(vec![])).await?;
        // Pong 응답 대기 및 타임아웃 처리
    }
}
```

#### Heartbeat 베스트 프랙티스
- **간격**: 20-30초 (대부분의 앱에 적합)
- **미응답 처리**: 연속 Pong 실패 시 연결 종료
- **리소스 관리**: 응답 없는 연결 정리로 리소스 낭비 방지

### 5.3 stream-tungstenite (자동 재연결 내장)

- **Lib.rs**: [stream-tungstenite](https://lib.rs/crates/stream-tungstenite)
- **특징**:
  - Exponential backoff 포함 다중 재시도 전략
  - 실시간 연결 상태 추적
  - 라이프사이클 이벤트 훅 시스템

---

## 6. 메시지 파싱 및 직렬화

### 6.1 Serde JSON 활용

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTicker {
    symbol: String,
    #[serde(deserialize_with = "de_float_from_str")]
    last_price: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    bid_price: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    ask_price: f64,
    last_update_id: u64,
}

// 문자열을 f64로 변환하는 커스텀 역직렬화
fn de_float_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
```

### 6.2 효율적인 파싱 전략

| 전략 | 장점 | 단점 |
|------|------|------|
| 직접 구조체 역직렬화 | 타입 안전, 빠름 | 스키마 변경 시 컴파일 오류 |
| `serde_json::Value` | 유연함 | 힙 할당 많음, 메모리 단편화 |
| Lazy 역직렬화 | 부분 파싱 가능, 10배 빠름 | 구현 복잡 |

### 6.3 바이너리 포맷 대안

고성능이 필요한 경우 JSON 대신 바이너리 포맷 고려:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"      # JSON
rmp-serde = "1.1"       # MessagePack
ciborium = "0.2"        # CBOR
```

---

## 7. 동시성 및 Backpressure 관리

### 7.1 Tokio 동시성 모델

```rust
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

async fn handle_multiple_streams() {
    let (tx, mut rx) = mpsc::channel(1000); // 버퍼 크기 제한

    // 각 연결에 대해 별도 태스크 생성
    for url in exchange_urls {
        let tx = tx.clone();
        tokio::spawn(async move {
            let (ws, _) = connect_async(url).await?;
            let (_, mut read) = ws.split();

            while let Some(msg) = read.next().await {
                if tx.send(msg?).await.is_err() {
                    break; // 수신자 종료됨
                }
            }
            Ok::<_, Error>(())
        });
    }
}
```

### 7.2 Backpressure 관리

#### Backpressure란?
다운스트림 컴포넌트가 데이터를 처리하는 속도보다 업스트림에서 데이터를 생산하는 속도가 빠를 때 발생하는 흐름 제어 문제.

#### 관리되지 않은 Backpressure의 결과
| 문제 | 영향 |
|------|------|
| 메모리 고갈 | 무한 버퍼 성장으로 크래시 |
| 지연시간 증가 | 큐에 쌓인 데이터가 오래됨 |
| 데이터 부정합 | 시간 민감 앱에서 구식 정보 처리 |
| 메시지 손실 | 버퍼 가득 차면 데이터 폐기 |
| 연쇄 장애 | 느린 클라이언트 하나가 전체 서버 불안정화 |

#### Backpressure 전략

```rust
use tokio::sync::mpsc;

// 1. 제한된 채널 버퍼 사용
let (tx, rx) = mpsc::channel(1000); // 1000개 초과 시 send 블록

// 2. try_send로 드롭 전략
if tx.try_send(msg).is_err() {
    metrics.increment("dropped_messages");
    // 또는 오래된 메시지 대신 새 메시지 우선
}

// 3. 타임아웃과 함께 전송
use tokio::time::timeout;
if timeout(Duration::from_millis(100), tx.send(msg)).await.is_err() {
    // 타임아웃 - 느린 소비자 감지
    handle_slow_consumer();
}
```

### 7.3 Arc를 활용한 공유 상태

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct SharedState {
    orderbook: RwLock<OrderBook>,
    positions: RwLock<HashMap<String, Position>>,
}

async fn process_message(state: Arc<SharedState>, msg: Message) {
    let mut orderbook = state.orderbook.write().await;
    orderbook.update(msg);
}
```

---

## 8. 권장 아키텍처

### 8.1 기본 의존성

```toml
[dependencies]
# WebSocket
tokio-tungstenite = { version = "0.26", features = ["rustls-tls-native-roots"] }

# 런타임
tokio = { version = "1", features = ["full"] }

# 직렬화
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 에러 처리
anyhow = "1.0"
thiserror = "2.0"

# 로깅
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 8.2 프로덕션 아키텍처 예시

```
┌─────────────────────────────────────────────────────────────┐
│                      Application                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Binance    │  │  Coinbase   │  │   Kraken    │         │
│  │  Stream     │  │  Stream     │  │   Stream    │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          ▼                                  │
│              ┌───────────────────────┐                      │
│              │  Normalized Channel   │ (mpsc, bounded)      │
│              └───────────┬───────────┘                      │
│                          ▼                                  │
│              ┌───────────────────────┐                      │
│              │   Message Processor   │                      │
│              │   (Backpressure Mgmt) │                      │
│              └───────────┬───────────┘                      │
│                          ▼                                  │
│              ┌───────────────────────┐                      │
│              │   Trading Strategy    │                      │
│              └───────────────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

### 8.3 핵심 설계 원칙

1. **연결당 하나의 태스크**: 각 WebSocket 연결을 별도의 Tokio 태스크로 관리
2. **제한된 채널**: 무한 버퍼 방지를 위해 항상 bounded channel 사용
3. **우아한 종료**: 시그널 처리 및 연결 정리 로직 구현
4. **메트릭 수집**: 지연시간, 메시지 수, 재연결 횟수 추적
5. **구조화된 로깅**: tracing 크레이트로 디버깅 용이성 확보

---

## 9. Binance WebSocket 연결 관리 상세

### 9.1 Ping/Pong 메커니즘

Binance WebSocket은 API 유형에 따라 다른 Ping/Pong 타이밍을 사용한다:

| API 유형 | Ping 간격 | Pong 타임아웃 |
|----------|-----------|---------------|
| **Spot WebSocket Streams** | 20초 | 1분 (2025년 변경) |
| **Spot WebSocket API** | 20초 | 1분 |
| **Futures WebSocket API** | 3분 | 10분 |

**중요**: 2025년 업데이트로 허용 Pong 지연이 10분에서 1분으로 변경됨

```rust
// Binance 권장 Ping/Pong 처리
async fn handle_binance_ping(ws: &mut WebSocket, payload: Vec<u8>) {
    // Ping 수신 시 즉시 동일한 payload로 Pong 응답
    ws.send(Message::Pong(payload)).await.unwrap();
}

// Unsolicited Pong (선제적 연결 유지)
async fn send_unsolicited_pong(ws: &mut WebSocket) {
    // Binance: "Unsolicited pong frames are allowed, but will not prevent disconnection"
    // 권장: payload가 비어있는 pong 전송
    ws.send(Message::Pong(vec![])).await.unwrap();
}
```

### 9.2 연결 제한

| 제한 항목 | 값 | 설명 |
|-----------|-----|------|
| **연결 시도** | 300회/5분/IP | 초과 시 차단 |
| **메시지 전송** | 5개/초 (Spot), 10개/초 (Futures) | 초과 시 연결 종료 |
| **스트림 구독** | 1024개/연결 | 단일 연결당 최대 |
| **Ping/Pong 레이트** | 5회/초 | 초과 시 연결 종료 |

### 9.3 구독/해제 메시지 포맷

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SubscribeMessage {
    method: String,
    params: Vec<String>,
    id: u64,
}

// 구독
let subscribe = SubscribeMessage {
    method: "SUBSCRIBE".to_string(),
    params: vec!["btcusdt@trade".to_string(), "ethusdt@depth@100ms".to_string()],
    id: 1,
};

// 해제
let unsubscribe = SubscribeMessage {
    method: "UNSUBSCRIBE".to_string(),
    params: vec!["btcusdt@trade".to_string()],
    id: 2,
};
```

---

## 10. JSON 파싱 최적화

### 10.1 라이브러리 성능 비교

Twitter JSON 파싱 벤치마크 ([sonic-rs](https://github.com/cloudwego/sonic-rs)):

| 라이브러리 | 역직렬화 시간 | 비고 |
|------------|---------------|------|
| **sonic_rs::from_slice_unchecked** | 694.74 µs | 가장 빠름, 유효성 검사 생략 |
| **sonic_rs::from_slice** | 796.44 µs | 유효성 검사 포함 |
| **simd_json::from_slice** | 1.0615 ms | - |
| **serde_json::from_slice** | 2.2659 ms | 가장 안정적 |

**직렬화 성능** (Twitter JSON):
| 라이브러리 | 직렬화 시간 |
|------------|-------------|
| sonic_rs::to_string | 380.90 µs |
| serde_json::to_string | 788.98 µs |
| simd_json::to_string | 965.66 µs |

### 10.2 sonic-rs가 빠른 이유

1. **직접 파싱**: simd-json은 JSON → tape → Rust 구조체 2단계, sonic-rs는 JSON → Rust 구조체 직접 파싱
2. **SIMD 최적화**: x86_64 및 aarch64에서 SIMD 명령어 활용
3. **임시 데이터 구조 없음**: 중간 표현(tape) 생략으로 메모리 절약

```toml
[dependencies]
sonic-rs = "0.3"

# 빌드 시 SIMD 활성화
# .cargo/config.toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

### 10.3 Lazy 역직렬화 구현

고성능 시나리오에서 전체 JSON을 파싱하지 않고 필요한 부분만 추출:

```rust
use sonic_rs::LazyValue;

// LazyValue로 부분 파싱
#[derive(Deserialize)]
struct PartialMessage<'a> {
    #[serde(borrow)]
    e: &'a str,  // 이벤트 타입만 먼저 확인
    #[serde(borrow)]
    data: LazyValue<'a>,  // 나머지는 지연 파싱
}

// 이벤트 타입에 따라 다른 구조체로 파싱
fn route_message(raw: &[u8]) -> Result<(), Error> {
    let partial: PartialMessage = sonic_rs::from_slice(raw)?;

    match partial.e {
        "trade" => {
            let trade: Trade = sonic_rs::from_str(partial.data.as_raw_str())?;
            handle_trade(trade)
        }
        "depthUpdate" => {
            let depth: DepthUpdate = sonic_rs::from_str(partial.data.as_raw_str())?;
            handle_depth(depth)
        }
        _ => Ok(())
    }
}
```

**성능 향상**: 이 접근법으로 패킷 라우팅 시간이 600ns → 60ns로 10배 개선 가능

### 10.4 라이브러리 선택 가이드

| 상황 | 권장 라이브러리 |
|------|-----------------|
| 일반적인 사용 | serde_json (안정성, 호환성) |
| 최고 성능 필요 | sonic-rs (x86_64/aarch64) |
| 차선 성능 + 안전성 | simd-json |
| 부분 파싱 필요 | sonic-rs LazyValue |

---

## 11. 에러 핸들링

### 11.1 WebSocket Close 코드

| 코드 | 이름 | 설명 | 처리 방법 |
|------|------|------|-----------|
| **1000** | Normal Closure | 정상 종료 | 재연결 불필요 |
| **1001** | Going Away | 서버 종료/재시작 | 백오프 후 재연결 |
| **1006** | Abnormal Closure | 비정상 종료 | 즉시 재연결 시도 |
| **1008** | Policy Violation | 정책 위반 | 로그 기록, 설정 확인 |
| **1009** | Message Too Big | 메시지 크기 초과 | 메시지 크기 조정 |
| **1011** | Unexpected Condition | 서버 내부 오류 | 백오프 후 재연결 |
| **1012** | Service Restart | 서비스 재시작 | 곧 재연결 |
| **1013** | Try Again Later | 일시적 과부하 | 지수 백오프 재연결 |

### 11.2 Binance 특화 에러 처리

```rust
use tungstenite::protocol::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;

fn handle_close(frame: Option<CloseFrame>) {
    match frame {
        Some(CloseFrame { code, reason }) => {
            match code {
                CloseCode::Normal => {
                    tracing::info!("정상 종료");
                }
                CloseCode::Policy => {
                    // Binance 1008: 과부하 또는 정책 위반
                    if reason.contains("Payload too long") {
                        tracing::warn!("메시지 크기 초과: {}", reason);
                    } else {
                        tracing::warn!("서버 과부하 또는 정책 위반: {}", reason);
                    }
                    // 지수 백오프로 재연결
                }
                CloseCode::Again => {
                    // 1013: 일시적 과부하
                    tracing::info!("서버 과부하, 재연결 대기");
                }
                _ => {
                    tracing::error!("예상치 못한 종료: {:?} - {}", code, reason);
                }
            }
        }
        None => {
            // 1006: 비정상 종료 (프레임 없음)
            tracing::warn!("비정상 연결 종료");
        }
    }
}
```

### 11.3 네트워크 불안정성 대응

```rust
use tokio::time::{timeout, Duration};
use anyhow::{Result, Context};

struct ReconnectConfig {
    initial_delay: Duration,
    max_delay: Duration,
    max_retries: u32,
    jitter_factor: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            max_retries: 10,
            jitter_factor: 0.5,
        }
    }
}

async fn connect_with_retry(url: &str, config: &ReconnectConfig) -> Result<WebSocket> {
    let mut delay = config.initial_delay;
    let mut attempts = 0;

    loop {
        match timeout(Duration::from_secs(10), connect_async(url)).await {
            Ok(Ok((ws, _))) => return Ok(ws),
            Ok(Err(e)) => {
                attempts += 1;
                if attempts >= config.max_retries {
                    return Err(e).context("최대 재시도 횟수 초과");
                }

                // 지터 추가
                let jitter = rand::random::<f64>() * config.jitter_factor;
                let actual_delay = delay.mul_f64(1.0 + jitter);

                tracing::warn!(
                    "연결 실패 (시도 {}/{}): {}. {}ms 후 재시도",
                    attempts, config.max_retries, e, actual_delay.as_millis()
                );

                tokio::time::sleep(actual_delay).await;
                delay = (delay * 2).min(config.max_delay);
            }
            Err(_) => {
                tracing::warn!("연결 타임아웃");
                attempts += 1;
            }
        }
    }
}
```

---

## 12. 실제 코드 예시

### 12.1 Binance 기본 연결

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use url::Url;

const BINANCE_WS_API: &str = "wss://stream.binance.com:9443";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TradeEvent {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    quantity: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = format!("{}/ws/btcusdt@trade", BINANCE_WS_API);
    let (mut ws, _) = connect_async(Url::parse(&url)?).await?;

    while let Some(msg) = ws.next().await {
        match msg? {
            Message::Text(text) => {
                let trade: TradeEvent = serde_json::from_str(&text)?;
                println!("{}: {} @ {}", trade.symbol, trade.quantity, trade.price);
            }
            Message::Ping(payload) => {
                ws.send(Message::Pong(payload)).await?;
            }
            Message::Close(frame) => {
                tracing::info!("연결 종료: {:?}", frame);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### 12.2 멀티 스트림 구독

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::json;

async fn multi_stream_client() -> anyhow::Result<()> {
    let url = "wss://stream.binance.com:9443/ws";
    let (mut ws, _) = connect_async(url).await?;

    // 여러 스트림 동시 구독
    let subscribe = json!({
        "method": "SUBSCRIBE",
        "params": [
            "btcusdt@trade",
            "ethusdt@trade",
            "btcusdt@depth@100ms"
        ],
        "id": 1
    });

    ws.send(Message::Text(subscribe.to_string().into())).await?;

    // 메시지 처리
    while let Some(msg) = ws.next().await {
        if let Message::Text(text) = msg? {
            // 스트림 종류에 따라 다르게 처리
            if text.contains("\"e\":\"trade\"") {
                // 거래 데이터 처리
            } else if text.contains("\"e\":\"depthUpdate\"") {
                // 호가창 데이터 처리
            }
        }
    }

    Ok(())
}
```

### 12.3 프로덕션 클라이언트 구조

```rust
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};

pub struct BinanceClient {
    config: ReconnectConfig,
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    is_connected: bool,
    subscriptions: Vec<String>,
    last_message_time: std::time::Instant,
}

impl BinanceClient {
    pub async fn run(
        &self,
        tx: mpsc::Sender<MarketData>,
    ) -> anyhow::Result<()> {
        loop {
            match self.connect_and_process(&tx).await {
                Ok(()) => {
                    tracing::info!("정상 종료");
                    break;
                }
                Err(e) => {
                    tracing::error!("연결 오류: {}", e);
                    // 재연결 로직
                    tokio::time::sleep(self.config.initial_delay).await;
                }
            }
        }
        Ok(())
    }

    async fn connect_and_process(
        &self,
        tx: &mpsc::Sender<MarketData>,
    ) -> anyhow::Result<()> {
        let (mut ws, _) = connect_async(&self.config.url).await?;

        // 구독 재설정
        let state = self.state.read().await;
        for sub in &state.subscriptions {
            let msg = format!(r#"{{"method":"SUBSCRIBE","params":["{}"],"id":1}}"#, sub);
            ws.send(Message::Text(msg.into())).await?;
        }
        drop(state);

        // Heartbeat 태스크
        let heartbeat_ws = ws.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            loop {
                interval.tick().await;
                if heartbeat_ws.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        });

        // 메시지 처리
        while let Some(msg) = ws.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<MarketData>(&text) {
                        if tx.send(data).await.is_err() {
                            break; // 수신자 종료
                        }
                    }
                }
                Message::Ping(payload) => {
                    ws.send(Message::Pong(payload)).await?;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        Ok(())
    }
}
```

---

## 13. 참고 자료

### 공식 문서 및 GitHub
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)
- [tokio-websockets GitHub](https://github.com/Gelbpunkt/tokio-websockets)
- [wtx Crates.io](https://crates.io/crates/wtx)
- [fastwebsockets GitHub](https://github.com/denoland/fastwebsockets)
- [barter-rs GitHub](https://github.com/barter-rs/barter-rs)
- [binance-connector-rust GitHub](https://github.com/binance/binance-connector-rust)
- [ezsockets GitHub](https://github.com/gbaranski/ezsockets)
- [sonic-rs GitHub](https://github.com/cloudwego/sonic-rs)

### 벤치마크 및 분석
- [The fastest WebSocket implementation](https://c410-f3r.github.io/thoughts/the-fastest-websocket-implementation/)
- [wtx Benchmark](https://bencher.dev/perf/wtx)
- [rustls-bench-results](https://github.com/aochagavia/rustls-bench-results)
- [Continuous benchmarking for rustls](https://ochagavia.nl/blog/continuous-benchmarking-for-rustls/)
- [serde-rs/json-benchmark](https://github.com/serde-rs/json-benchmark)
- [web-socket-benchmark](https://github.com/nurmohammed840/web-socket-benchmark)

### TLS 및 보안
- [rustls GitHub](https://github.com/rustls/rustls)
- [aws-lc-rs GitHub](https://github.com/aws/aws-lc-rs)
- [Rustls on track to outperform OpenSSL](https://www.memorysafety.org/blog/rustls-performance/)
- [Introducing AWS Libcrypto for Rust](https://aws.amazon.com/blogs/opensource/introducing-aws-libcrypto-for-rust-an-open-source-cryptographic-library-for-rust/)

### 메모리 할당자
- [Optimizing Rust Performance with jemalloc](https://leapcell.medium.com/optimizing-rust-performance-with-jemalloc-c18057532194)
- [Understanding jemalloc](https://leapcell.io/blog/understanding-jemalloc)

### JSON 파싱 최적화
- [sonic-rs Documentation](https://docs.rs/sonic-rs)
- [simd-json Documentation](https://docs.rs/simd-json)
- [Surprises in the Rust JSON Ecosystem](https://ecton.dev/rust-json-ecosystem/)
- [Inside Serde: Building a Custom JSON Deserializer](https://totodore.github.io/serde-wrapper/)

### Binance API 문서
- [Binance WebSocket Streams](https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams)
- [Binance WebSocket API General Info](https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-api-general-info)
- [Binance WebSocket Limits](https://academy.binance.com/en/articles/what-are-binance-websocket-limits)

### 튜토리얼
- [WebSocket Implementation in Rust - WebSocket.org](https://websocket.org/guides/languages/rust/)
- [Easily connect to Binance WebSocket streams with Rust](https://tms-dev-blog.com/easily-connect-to-binance-websocket-streams-with-rust/)
- [Building Real-Time Binance WebSocket Clients in Rust with Tokio](https://medium.com/@ekfqlwcjswl/building-real-time-binance-websocket-clients-in-rust-with-tokio-2e0027f0f1fd)
- [Building a Real-Time Market Data Client in Rust with WebSockets](https://medium.com/@onur-karaduman/building-a-real-time-market-data-client-in-rust-with-websockets-b5b9d798c0d9)
- [Rust WebSocket: Building Real-Time Applications in 2025](https://www.videosdk.live/developer-hub/websocket/rust-websocket)

### WebSocket 에러 처리
- [WebSocket Close Codes Reference](https://websocket.org/reference/close-codes/)
- [WebSocket Close Codes Explained](https://dev.to/_saranshbarua/websocket-close-codes-explained-a-guide-to-connection-status-codes-5cmd)

### Backpressure 및 동시성
- [Backpressure in WebSocket Streams](https://skylinecodes.substack.com/p/backpressure-in-websocket-streams)
- [WebSocket Best Practices for Production Applications](https://lattestream.com/blog/websocket-best-practices)

### 암호화폐 관련
- [barter-data Documentation](https://docs.rs/barter-data)
- [crypto-ws-client Documentation](https://docs.rs/crypto-ws-client)
- [crypto-botters - Lib.rs](https://lib.rs/crates/crypto-botters)
- [reconnecting-websocket Crate](https://crates.io/crates/reconnecting-websocket)
- [ezsockets - Lib.rs](https://lib.rs/crates/ezsockets)

---

*마지막 업데이트: 2026-01-16*
