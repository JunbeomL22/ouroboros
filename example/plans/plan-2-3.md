# 기술 인프라 요구사항 보고서 작성 계획 (수정본)

## 목표
크립토시장.md와 연계하여 기술 인프라 요구사항 보고서를 보강

## 이전 실패 원인 및 해결 방안

| 문제점 | 해결 방안 |
|--------|----------|
| 크립토시장.md 연계 너무 간략 | API Rate Limit, 수수료를 기술 설계와 명시적 연결 |
| Rust 코드 예시 부재 | Heartbeat, Exponential Backoff, WebSocket 재연결 코드 추가 |
| 오더북 동기화 추상적 | 스냅샷 → 델타 → 시퀀스 검증 → 갭 복구 순서도 추가 |
| 아키텍처 다이어그램 없음 | ASCII 다이어그램 추가 |
| 전략별 기술 스택 조합 미제시 | 트레이딩 빈도별 권장 스택 표 추가 |

## 실행 단계

### 1단계: 웹 검색 - API/레이턴시 심층 조사
병렬 검색 3회:
- Binance WebSocket reconnection exponential backoff 2025
- crypto exchange order book snapshot synchronization algorithm
- AWS colocation crypto trading latency 2025

### 2단계: 웹 검색 - OMS/리스크 관리 조사
병렬 검색 3회:
- Rust crypto trading OMS implementation 2025
- crypto margin calculation formula isolated cross
- crypto trading system heartbeat implementation

### 3단계: 웹 검색 - 데이터 파이프라인 조사
병렬 검색 2회:
- QuestDB crypto orderbook ingestion performance 2025
- on-chain whale tracking API integration 2025

### 4단계: 보고서 보강 내용 작성
기존 기술.md에 다음 섹션 추가/보강:

1. **크립토시장.md 연계 강화**
   - Binance 300연결/5분 → 연결 풀 150개 설계 근거
   - OKX 3회/초 → 요청 큐잉 전략
   - dYdX BTC/SOL 무료 → 차익거래 비용 계산

2. **Rust 코드 예시 추가**
   - WebSocket Heartbeat 관리
   - Exponential Backoff 재연결
   - 연결 풀 관리

3. **오더북 동기화 프로세스 상세화**
   ```
   [REST 스냅샷] → [WS 델타 적용] → [시퀀스 검증] → [갭 복구]
   ```

4. **데이터 파이프라인 아키텍처 다이어그램**
   ```
   [Exchange WS] → [Normalizer] → [QuestDB] → [Analytics]
                        ↓
               [Order Book Engine]
   ```

5. **전략 유형별 기술 스택 권장표**
   - 마켓 메이킹 (1000+ 주문/초)
   - 차익거래 (10-100 주문/초)
   - 트렌드 팔로잉 (1-10 주문/분)

### 5단계: 기술.md 파일 업데이트
Edit 도구로 기존 파일 보강 (Write로 전체 덮어쓰기 X)

## 산출물
- 위치: `/home/junbeom/Projects/ouroboros/example/기술.md`
- 보강 내용: 코드 예시, 다이어그램, 연계 강화, 전략별 스택