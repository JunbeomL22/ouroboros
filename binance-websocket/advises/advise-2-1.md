## 리뷰 피드백

### 보강 필요 사항

**1. TLS 섹션 보완 (4.2)**
- rustls 사용 시 `aws-lc-rs` 암호화 백엔드를 명시적으로 권장해야 함. 벤치마크 결과 `ring`보다 `aws-lc-rs`가 처리량이 2배 이상 높음
- jemalloc 할당자 사용 권장사항 추가 필요 (나가는 데이터 전송 처리량이 2배 이상 증가)

**2. Binance 연결 관리 세부사항 누락 (5장)**
- Binance 서버가 3분마다 Ping 프레임 전송, 10분 내 Pong 미응답 시 연결 종료됨을 명시해야 함
- 클라이언트 측 unsolicited pong 프레임 전송(15분보다 짧은 간격) 허용됨
- IP당 5분간 300회 연결 시도 제한 명시 필요
- 초당 5개 메시지 제한 (Ping/Pong/subscribe/unsubscribe 포함)
- Close code 1008 (서버 과부하) 처리 전략 추가

**3. 메시지 파싱 성능 최적화 (6장)**
- `simd_json` 또는 `sonic-rs` 라이브러리 언급 필요. 특정 CPU 타겟팅 가능 시 `simd_json`이 serde_json보다 월등히 빠름
- 2단계 역직렬화 회피 전략 구체화 필요: 메시지 라우팅용 부분 파싱 → 핸들러별 전체 파싱
- Lazy 역직렬화의 구체적 구현 예시 추가 권장

**4. barter-data 지원 거래소 목록 업데이트 필요 (3.2)**
- Bybit (Spot/Perpetuals), Gate.io (Spot/Perpetuals/Options), Bitmex 추가 지원됨

**5. fastwebsockets 주의사항 추가 (2.2)**
- 일부 벤치마크에서 "unsound and not thread-safe", "non-strict spec compliance" 지적됨. 프로덕션 사용 시 주의 필요

**6. 백오프 전략 수정 (5.1)**
- reconnecting-websocket 크레이트 존재 확인 필요. 대부분의 프로젝트에서 직접 구현하거나 tokio-retry 사용

### 구조적 제안

**7. 에러 핸들링 섹션 추가**
- WebSocket 에러 코드별 처리 전략 (1000, 1008 등)
- 네트워크 불안정성 대응

**8. 실제 코드 예시 보강**
- 완전한 재연결 로직 구현 예시
- Binance 스트림 구독/해제 메시지 포맷

### 정확성 검토

**9. 벤치마크 컨텍스트**
- 다른 벤치마크(web-socket-benchmark)에서는 fastwebsockets가 tokio-tungstenite보다 2배 빠름. 벤치마크 방법론에 따라 결과가 크게 다를 수 있음을 명시

**10. tokio-tungstenite 버전**
- 현재 최신 버전 0.28.x, 보고서의 0.26 버전 업데이트 필요