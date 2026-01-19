# Rust WebSocket for Crypto Market - Best Practices & Libraries Research Plan

## 목표
Rust에서 암호화폐 시장 WebSocket 연결을 위한 최신 라이브러리와 모범 사례를 조사하여 보고서 작성

## 조사 항목

### 1. TLS 성능 최적화
- rustls의 `aws-lc-rs` vs `ring` 암호화 백엔드 벤치마크
- jemalloc 할당자 성능 이점

### 2. Binance WebSocket 연결 관리 상세
- Ping/Pong 프레임 타이밍 (3분 Ping, 10분 Pong 타임아웃)
- Unsolicited pong 프레임 전략
- 연결 제한: IP당 5분간 300회, 초당 5개 메시지
- Close code 1008 (서버 과부하) 처리

### 3. JSON 파싱 최적화
- `simd_json` vs `sonic-rs` vs `serde_json` 성능 비교
- 2단계 역직렬화 회피: 부분 파싱 → 핸들러별 전체 파싱
- Lazy 역직렬화 구현 방식

### 4. 라이브러리 업데이트
- barter-data 지원 거래소 (Bybit, Gate.io, Bitmex 추가 확인)
- fastwebsockets 안전성 이슈 (unsound, thread-safety)
- tokio-tungstenite 최신 버전 (0.28.x)

### 5. 에러 핸들링
- WebSocket 에러 코드별 처리 (1000, 1008 등)
- 네트워크 불안정성 대응 전략

### 6. 벤치마크 방법론
- web-socket-benchmark 등 다양한 벤치마크 결과 비교
- 방법론에 따른 결과 차이 명시

### 7. 실제 코드 예시
- 재연결 로직 구현
- Binance 구독/해제 메시지 포맷

## 웹 검색 쿼리 계획
1. "rustls aws-lc-rs ring benchmark performance 2025"
2. "rust jemalloc websocket performance"
3. "binance websocket ping pong timeout connection limits"
4. "simd_json sonic-rs serde_json benchmark rust"
5. "barter-data rust crypto supported exchanges 2025"
6. "fastwebsockets rust safety issues thread-safe"
7. "tokio-tungstenite latest version 2025"
8. "rust websocket benchmark comparison 2025"
9. "rust crypto websocket best practices production"