# Plan Review: Binance WebSocket 샘플 프로젝트

## 잠재적 문제점 및 개선 사항

### 1. 디렉토리 이름 오타
- Task에서 `webseocket/`로 명시되어 있으나, 계획에서는 `websocket/`로 작성됨
- **확인 필요**: 사용자 의도가 `websocket/`인지 `webseocket/`인지 명확히 해야 함

### 2. API 문서 미반영 가능성
- `api.md`와 `rust-websocket.md` 파일 내용을 실제로 읽어서 반영했는지 불분명
- Binance WebSocket API의 실제 스펙(메시지 포맷, 레이트 리밋, 인증 요구사항 등)이 계획에 정확히 반영되어야 함

### 3. Ping/Pong 스펙 검증 필요
- "Spot: 20초 Ping, 1분 Pong 타임아웃"이라고 했으나, 실제 Binance 문서와 일치하는지 확인 필요
- Binance는 서버가 Ping을 보내고 클라이언트가 Pong을 응답하는 방식인지, 또는 클라이언트가 Ping을 보내야 하는지 명확히 해야 함

### 4. Combined Stream 엔드포인트 누락
- 단일 스트림 URL만 언급됨: `wss://stream.binance.com:9443/ws/btcusdt@trade`
- Combined stream 엔드포인트 (`wss://stream.binance.com:9443/stream?streams=`) 고려 필요

### 5. 메시지 타입 추가 고려
- `TradeEvent`, `BookTicker`, `DepthUpdate`만 정의
- 구독 응답 메시지, 에러 응답 메시지 타입도 필요
- `result`, `id` 필드를 포함한 응답 처리 로직 필요

### 6. 재연결 로직 세부사항 부족
- 지수 백오프의 초기값, 최대값, jitter 적용 여부 미정의
- 최대 재연결 시도 횟수 제한 필요
- 연결 상태 추적 메커니즘 미언급

### 7. 구독 관리 상태
- 재연결 시 기존 구독 복원 로직 언급 없음
- 구독 ID 추적 및 응답 매칭 로직 필요

### 8. 테스트 코드 계획 없음
- 단위 테스트, 통합 테스트 계획 없음
- 최소한 메시지 파싱 테스트는 포함해야 함

### 9. 의존성 버전 고정
- `rand = "0.8"`이지만 현재 최신은 0.9.x
- 프로젝트 요구사항에 맞는 버전인지 확인 필요

### 10. Graceful Shutdown 구체화 부족
- "우아한 종료" 언급만 있고 구체적 구현 방법 미정의
- WebSocket close frame 전송, 진행 중인 작업 완료 대기 등 명시 필요

## 권장 수정사항

1. 원본 문서(`rust-websocket.md`, `api.md`) 내용을 기반으로 스펙 검증
2. 구독 상태 관리 모듈 추가 (`src/subscription.rs`)
3. 재연결 파라미터 구체화 (초기 1초, 최대 60초, jitter ±10%)
4. 메시지 파싱 테스트 코드 추가
5. 디렉토리 이름 확정