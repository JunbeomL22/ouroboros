# Binance WebSocket API 종합 보고서

## 목차
1. [개요](#개요)
2. [WebSocket 엔드포인트](#websocket-엔드포인트)
3. [스트림 유형](#스트림-유형)
4. [연결 제한 및 Rate Limit](#연결-제한-및-rate-limit)
5. [연결 유지 및 Heartbeat](#연결-유지-및-heartbeat)
6. [다중 WebSocket 연결과 성능](#다중-websocket-연결과-성능)
7. [이중화(Redundancy) 전략](#이중화redundancy-전략)
8. [지연시간(Latency) 최적화](#지연시간latency-최적화)
9. [SBE 마켓 데이터 스트림](#sbe-마켓-데이터-스트림)
10. [베스트 프랙티스](#베스트-프랙티스)
11. [참고 자료](#참고-자료)

---

## 개요

Binance WebSocket API는 실시간 시장 데이터와 계정 업데이트를 제공하는 양방향 통신 채널이다. REST API의 폴링 방식과 달리, WebSocket은 서버가 클라이언트에게 데이터를 푸시하여 지연시간을 최소화한다.

---

## WebSocket 엔드포인트

### Spot (현물)

| 용도 | 엔드포인트 |
|------|-----------|
| 메인 스트림 | `wss://stream.binance.com:9443/` |
| 마켓 데이터 전용 | `wss://data-stream.binance.vision` |
| WebSocket API | `wss://ws-api.binance.com:443/ws-api/v3` |
| SBE 마켓 데이터 | `wss://stream-sbe.binance.com:9443` |
| 테스트넷 | `wss://stream.testnet.binance.vision:9443/` |

> **주의**: `wss://data-stream.binance.vision`은 마켓 데이터만 제공하며, User Data Stream은 사용 불가.

### USD-M Futures (USDT 선물, fstream)

| 용도 | 엔드포인트 |
|------|-----------|
| 메인 스트림 | `wss://fstream.binance.com/` |
| WebSocket API | `wss://ws-fapi.binance.com/ws-fapi/v1` |
| 테스트넷 | `wss://stream.binancefuture.com` |

### Coin-M Futures (코인 선물, dstream)

| 용도 | 엔드포인트 |
|------|-----------|
| 메인 스트림 | `wss://dstream.binance.com/` |
| 테스트넷 | `wss://dstream.binancefuture.com` |

> **주의**: Raw stream 형식(`/ws?<streamName>`)은 지원하지 않음. 예: `wss://fstream.binance.com/ws?btcusdt@depth`는 무효.

---

## 스트림 유형

### 1. bookTicker (호가 최우선가)
- **설명**: 최우선 매수/매도 호가의 가격 및 수량 변경을 실시간 푸시
- **스트림 이름**: `<symbol>@bookTicker`
- **데이터**: updateId, symbol, 최우선 매수가/수량, 최우선 매도가/수량
- **용도**: 가장 낮은 지연시간의 호가 정보 필요시

### 2. depth (호가창)
- **설명**: 상위 N개 호가 정보 (5, 10, 20 레벨)
- **스트림 이름**: `<symbol>@depth<levels>` 또는 `<symbol>@depth<levels>@100ms`
- **용도**: 호가창 시각화, 오더북 분석

### 3. trade (개별 체결)
- **설명**: 각 체결 정보를 개별적으로 푸시 (각 체결은 고유한 buyer/seller 보유)
- **스트림 이름**: `<symbol>@trade`
- **데이터**: tradeId, price, quantity, tradeTime, isBuyerMaker

### 4. aggTrade (집계 체결)
- **설명**: 동일 시간, 동일 주문, 동일 가격의 체결을 집계
- **스트림 이름**: `<symbol>@aggTrade`
- **용도**: 체결 데이터 양 축소, 네트워크 효율화

### 5. kline (캔들스틱)
- **설명**: 지정 간격의 캔들스틱 데이터
- **스트림 이름**: `<symbol>@kline_<interval>`
- **간격**: 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M
- **타임존**: UTC+0 기본, UTC+8 옵션 가능

### 6. ticker (24시간 롤링 통계)
- **설명**: 24시간 롤링 윈도우 통계 (UTC 일별 통계 아님)
- **스트림 이름**: `<symbol>@ticker`
- **미니 버전**: `<symbol>@miniTicker`

> **참고**: 심볼명은 소문자로 작성해야 함. 예: `bnbbtc@aggTrade`, `btcusdt@bookTicker`

---

## 연결 제한 및 Rate Limit

### 연결 제한

| 항목 | Spot | Futures |
|------|------|---------|
| IP당 연결 시도 | 5분당 300회 | 5분당 300회 |
| 연결당 스트림 수 | 최대 1024개 | 최대 1024개 |
| **권장 스트림 수** | - | **200개 이하** |
| 수신 메시지 제한 | 초당 5개 | 초당 10개 |
| 연결 비용 | 2 weight | - |

> **중요**: Futures 공식 문서에서 "A single connection is not recommended to listen to more than 200 streams" 명시

### 구체적 권장 수치

| 항목 | 권장값 | 근거 |
|------|--------|------|
| **bookTicker 스트림/연결** | ~100개 | Futures 권장(200)의 절반으로 안전 마진 확보 |
| **IP당 동시 연결** | ~20개 | 5분당 300회 제한, 재연결 여유분 확보 |
| **이중화 연결** | 2-3개/스트림 | Kraken 공식 권장 기준 |

### Weight 제한
- Weight는 IP 주소별로 누적
- 해당 IP의 모든 연결이 공유
- 제한 초과 시 HTTP 429 응답
- 반복 위반 시 IP 차단 (2분 ~ 3일)

### 위반 시 처리
1. 429 응답 수신 → 요청 중단 및 백오프
2. 418 응답 → IP 차단 상태
3. 반복 위반 → 차단 기간 점진적 증가

---

## 연결 유지 및 Heartbeat

### Spot WebSocket
- 서버가 **20초마다** Ping 프레임 전송
- **1분 이내** Pong 응답 필요 (미응답 시 연결 해제)
- Pong의 payload는 Ping과 동일해야 함

### Futures WebSocket
- 서버가 **3분마다** Ping 프레임 전송
- **10분 이내** Pong 응답 필요 (미응답 시 연결 해제)
- 자발적 Pong (15분보다 짧은 간격) 허용

### 자동 연결 해제
- **24시간 후** 서버가 연결 자동 종료
- 클라이언트는 재연결 로직 필수 구현

---

## 다중 WebSocket 연결과 성능

### "여러 WebSocket을 띄우고 선택해야 한다"는 말의 의미

이 개념은 **Binance 한정이 아닌 모든 암호화폐 거래소에 해당**한다.

#### 1. 동일 거래소의 다중 연결 (로드 밸런싱)

**문제**: 단일 WebSocket 연결은 네트워크 상태, 서버 부하에 따라 지연시간이 변동한다.

**Binance 개발자 포럼 확인 수치**:

| 조건 | bookTicker 지연시간 |
|------|-------------------|
| 정상 상태 (단일 연결 기준) | 20-60ms 기본 |
| 최적 조건 (Tokyo VPS) | 5-10ms |
| 고변동성 시장 | 100-900ms 스파이크 |
| 과부하 상태 (60-70개 스트림 구독) | 8,000ms ~ 500,000ms |

**해결책**:
- 동일 스트림에 대해 **복수의 WebSocket 연결**을 유지
- 각 연결의 지연시간을 측정하여 **가장 빠른 연결의 데이터를 사용**
- 연결 장애 발생 시 즉시 백업 연결로 전환

```
연결 A: stream.binance.com → 평균 5ms 지연
연결 B: stream.binance.com → 평균 8ms 지연
연결 C: stream.binance.com → 평균 3ms 지연 ← 이 연결 사용
```

**Binance 특이사항**:
- Binance는 CDN/로드밸런서를 사용하여 연결을 분산
- 연결마다 다른 백엔드 서버에 라우팅될 수 있음
- 서버 과부하 시 close code 1008 발생 가능

#### 2. 스트림 분산 (권장 사항)

**문제**: 하나의 연결에 너무 많은 스트림을 구독하면:
- 메시지 처리 지연 증가
- 단일 장애점 발생
- 수신 버퍼 오버플로우 위험

**Coinbase 공식 문서 권장**:
> "BTC-USD와 ETH-USD 같은 고볼륨 상품을 같은 연결에 구독하지 말고, 별도의 WebSocket 연결로 분리하라"

**권장 구조**:
```
연결 1: BTC 관련 스트림 (btcusdt@bookTicker, btcusdt@trade)
연결 2: ETH 관련 스트림 (ethusdt@bookTicker, ethusdt@trade)
연결 3: 기타 알트코인 스트림
```

#### 3. 이 현상이 Binance만 해당하는가?

**아니오, 모든 암호화폐 거래소에 해당한다.**

| 거래소 | 특징 |
|--------|------|
| Binance | 아시아 리전 서버, CDN 분산, 연결별 지연시간 편차 존재 |
| Coinbase | US East 리전, 고빈도 채널 분리 권장 |
| Bybit | 유사한 구조, 멀티플렉싱 지원 |
| OKX | 복수 엔드포인트 제공 |
| Kraken | 복수 연결 이중화 공식 권장 |

**클라우드 인프라의 특성**:
- CDN 엣지 노드가 요청을 수신하지만, 실제 처리는 원본 서버에서 수행
- 로드밸런서가 요청을 다른 AZ(Availability Zone)의 서버로 라우팅 가능
- Ping 지연시간과 실제 WebSocket 지연시간이 다를 수 있음

---

## 이중화(Redundancy) 전략

### Kraken 공식 문서 인용

> "Using multiple (two or more) redundant WebSocket connections is usually the most effective solution to unexpected WebSocket disconnections, because it allows the market data feeds to continue uninterrupted regardless of how frequently the underlying connections are terminated."
>
> — [Kraken WebSocket API v1 - Unexpected Disconnections](https://support.kraken.com/articles/360044504011-websocket-api-v1-unexpected-disconnections-from-market-data-feeds)

### 이중화 구현 방식

| 방식 | 설명 | 장점 | 단점 |
|------|------|------|------|
| **Active-Standby** | 주 연결 실패 시 대기 연결 활성화 | 리소스 효율적 | 전환 시 지연 발생 |
| **Active-Active** | 모든 연결에서 데이터 수신, 최빠른 값 선택 | 지연시간 최소화 | 리소스 2배 사용 |
| **N+1 Redundancy** | N개 필요 시 N+1개 유지 | 고가용성 보장 | 관리 복잡성 증가 |

### 권장 이중화 아키텍처

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  WebSocket #1   │     │  WebSocket #2   │     │  WebSocket #3   │
│  (Primary)      │     │  (Secondary)    │     │  (Tertiary)     │
│  동일 스트림     │     │  동일 스트림     │     │  동일 스트림     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         ▼                       ▼                       ▼
    ┌──────────────────────────────────────────────────────────┐
    │                      Data Selector                       │
    │  - 타임스탬프 비교 (Event Time 기준)                       │
    │  - 도착 순서 기반 선택                                     │
    │  - 연결 건강도 모니터링                                    │
    └──────────────────────────────────────────────────────────┘
```

### 실제 적용 효과

차익거래 트레이더 사례:
- 인프라 최적화 전: 89ms 지연, 차익거래 성공률 23%
- 인프라 최적화 후: 42ms 지연, 차익거래 성공률 **61%**
- 연간 약 **$180,000** 기회손실 방지

---

## 지연시간(Latency) 최적화

### AWS 리전별 Binance 지연시간

연구 결과에 따르면, Binance API 지연시간은 다음 순서로 우수:

| 순위 | AWS 리전 | 평균 지연시간 | P99 지연시간 | 비고 |
|------|----------|-------------|-------------|------|
| 1위 | Osaka (ap-northeast-3) | 최저 | - | 24시간 테스트 결과 최우수 |
| 2~3위 | Tokyo (ap-northeast-1) | **~4ms** | **<13ms** | 상황에 따라 Seoul과 경쟁 |
| 2~3위 | Seoul (ap-northeast-2) | Tokyo와 유사 | - | 상황에 따라 Tokyo와 엎치락뒤치락 |

> **참고**: 24시간 테스트 결과 Osaka가 가장 우수했으며, Seoul과 Tokyo는 근소한 차이로 2~3위를 경쟁. 단정적 순위보다는 상황에 따라 변동됨.

### 지연시간 수준별 인프라

| 수준 | 지연시간 | 방법 |
|------|----------|------|
| 공용 인터넷 | 10-100+ ms | 일반 VPS/가정용 연결 |
| 동일 AZ VPS | 1-5 ms | AWS/GCP 아시아 리전 |
| 코로케이션 | 50-200 µs | 동일 데이터센터 |
| 전용 FIX 게이트웨이 | 10-50 µs | 기관 투자자용 |

### 최적화 전략

1. **서버 위치**: Tokyo/Osaka/Seoul AWS 리전 사용
2. **단일 AZ 유지**: 크로스-AZ 트래픽은 추가 지연 발생
3. **Cluster Placement Group**: 동일 CPG 내 인스턴스 배치 시 35-37% 지연 감소
4. **수신 버퍼 최대화**: WebSocket 수신 버퍼 크기를 가능한 한 크게 설정
5. **콜백 내 I/O 최소화**: 메시지 큐잉 후 별도 스레드에서 처리

### 현실적 고려사항

```
전략 유형          | 필요 지연시간
------------------|-------------
포트폴리오 모니터링  | 1-5초 충분
스윙 트레이딩       | 100ms-1초
데이 트레이딩       | 10-100ms
알고리즘 트레이딩   | 1-10ms
HFT               | < 1ms
```

> **중요**: 거래소 처리 시간이 5-10ms이므로, 클라이언트 코드를 50µs에서 10µs로 최적화해도 체감 효과 미미

---

## SBE 마켓 데이터 스트림

### 개요

**SBE (Simple Binary Encoding)**는 저지연 금융 애플리케이션을 위한 바이너리 메시지 인코딩 형식이다.

- **출시일**: 2025년 3월 18일
- **엔드포인트**: `wss://stream-sbe.binance.com:9443`
- **특징**: JSON 대비 82% 메모리 절약, 더 낮은 지연시간

### 기술적 장점

| 항목 | JSON | SBE |
|------|------|-----|
| 형식 | 텍스트 (사람이 읽을 수 있음) | 바이너리 (컴팩트) |
| 파싱 오버헤드 | 높음 | 최소 |
| 메모리 사용량 | 기준 | **82% 감소** |
| 타임스탬프 | 밀리초 | **마이크로초** |

### 접근 요건

| 항목 | 요건 |
|------|------|
| API Key | 필수 (X-MBX-APIKEY 헤더) |
| Key 유형 | **Ed25519만 허용** |
| 연결 유효기간 | 24시간 |

### Auto-culling 기능

고부하 상태에서 오래된 이벤트를 자동으로 삭제:

```
시간 T1의 이벤트 (대기 중)
시간 T2의 이벤트 (신규 생성, T2 > T1)
→ T1 이벤트 삭제, T2 이벤트만 전송
```

이는 심볼별로 적용되며, 지연된 오래된 데이터 대신 최신 데이터를 우선 전달.

### 2025년 주요 업데이트

| 날짜 | 변경 사항 |
|------|----------|
| 2025-03-18 | SBE Market Data Streams 출시 |
| 2025-11-26 | depth 스트림 업데이트 속도 50ms로 변경 (데이터량 최대 2배) |
| 2025-12-18 | FIX SBE 지원 추가 |

---

## 베스트 프랙티스

### 연결 관리

1. **지수 백오프 재연결**: 연결 실패 시 점진적으로 대기 시간 증가
2. **기존 연결 재사용**: 새 연결보다 기존 연결에 스트림 추가
3. **Combined Stream 사용**: 개별 스트림보다 결합 스트림으로 연결 수 감소
4. **5초 내 구독 전송**: 연결 후 즉시 구독 메시지 전송

### 데이터 처리

1. **REST 폴링 대신 WebSocket**: Rate Limit 위반의 주 원인은 REST 폴링
2. **User Data Stream 활용**: 계정 변경은 REST 대신 WebSocket으로 수신
3. **마이크로초 타임스탬프**: URL에 `timeUnit=MICROSECOND` 추가로 정밀도 향상
4. **UTF-8 처리**: 일부 심볼에 비-ASCII 문자 포함 가능

### 인증

1. **API Key**: `X-MBX-APIKEY` 헤더에 포함
2. **Signature**: `timestamp` 파라미터 필수, `recvWindow`로 유효 기간 설정
3. **2026-01-15 변경사항**: Signature 계산 전 payload를 percent-encode 해야 함
4. **Ed25519 권장**: 더 안전하고 성능이 우수함 (특히 SBE 사용 시 필수)

### 오류 처리

| 코드 | 의미 | 대응 |
|------|------|------|
| 1008 | 서버 과부하 | 백오프 후 재시도 |
| 429 | Rate Limit 초과 | 요청 중단, 대기 |
| 418 | IP 차단 | 차단 해제까지 대기 |

### 고변동성 시장 대응

Binance 공식 지침:
- 평상시 주문 상태 불확실 구간: **1초 이하**
- 고변동성 시: **최대 10초**
- RESTful 대신 **WebSocket User Data Stream 사용 권장**

---

## 참고 자료

### 공식 문서
- [Binance Spot WebSocket Streams](https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams)
- [Binance Spot WebSocket API](https://developers.binance.com/docs/binance-spot-api-docs/web-socket-api)
- [Binance Futures WebSocket API](https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-api-general-info)
- [Binance WebSocket Rate Limits](https://developers.binance.com/docs/binance-spot-api-docs/websocket-api/rate-limits)
- [Binance Academy - WebSocket Limits](https://academy.binance.com/en/articles/what-are-binance-websocket-limits)
- [Binance SBE Market Data](https://developers.binance.com/docs/binance-spot-api-docs/sbe-market-data-streams)

### GitHub 레포지토리
- [binance-spot-api-docs](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md)
- [binance-official-api-docs](https://github.com/binance-exchange/binance-official-api-docs/blob/master/web-socket-streams.md)
- [python-binance](https://python-binance.readthedocs.io/en/latest/websockets.html)

### 성능 분석
- [Binance Latency Analysis Across AWS Regions](https://viktoriatsybko.substack.com/p/an-analysis-of-binance-exchange-across)
- [AWS - Optimize tick-to-trade latency](https://aws.amazon.com/blogs/web3/optimize-tick-to-trade-latency-for-digital-assets-exchanges-and-trading-platforms-on-aws/)
- [Geographic Latency in Crypto](https://elitwilliams.medium.com/geographic-latency-in-crypto-how-to-optimally-co-locate-your-aws-trading-server-to-any-exchange-58965ea173a8)
- [Deltix WebSocket Data Feed Latency](https://ember.deltixlab.com/docs/performance/ws-market-data/)
- [Binance Latency Monitor](https://api.moon-bot.com/latency/)

### 이중화 및 모범사례
- [Kraken WebSocket API FAQ](https://support.kraken.com/articles/360022326871-kraken-websocket-api-frequently-asked-questions)
- [Kraken WebSocket Disconnections](https://support.kraken.com/articles/360044504011-websocket-api-v1-unexpected-disconnections-from-market-data-feeds)
- [Coinbase WebSocket Best Practices](https://docs.cloud.coinbase.com/exchange/docs/websocket-best-practices)
- [CoinAPI Latency Guide](https://www.coinapi.io/blog/crypto-trading-latency-guide)

### 멀티 거래소 라이브러리
- [Cryptofeed (Python)](https://github.com/bmoscon/cryptofeed)
- [CCXWS (JavaScript)](https://github.com/altangent/ccxws)

### 커뮤니티
- [Binance Developer Community](https://dev.binance.vision/)
- [bookTicker High Delay Discussion](https://dev.binance.vision/t/bookticker-high-delay-on-volatile-market/16010)
- [Multiple WebSocket Connections Implementation](https://dev.binance.vision/t/binance-api-multiple-websocket-connections-implemention/19280)
