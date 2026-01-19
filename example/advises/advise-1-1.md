# 계획 검토 의견

## 문제점

### 1. 실행 방법 부재
계획이 "조사 완료", "분석 완료"로 표시되어 있으나, **어떻게** 정보를 수집할지 구체적인 방법이 없음. 웹 검색 쿼리, 참조할 공식 문서, API 문서 링크 등이 명시되어야 함.

### 2. 검색 전략 미흡
- 각 거래소별 공식 수수료 페이지 URL 명시 필요
- API rate limit은 공식 문서에서만 정확한 정보 획득 가능
- 규제 정보는 최신 자료(2024-2025년) 확인 필수

### 3. 구체적 데이터 포인트 누락
수집해야 할 항목 명확화 필요:
- 수수료: Maker/Taker fee tier별 비율, VIP 레벨 조건
- API: 분당/초당 요청 제한, WebSocket 연결 수, 주문 제한
- Funding rate: 정산 주기(8시간/1시간), 계산 공식

### 4. 보고서 구조 미정의
최종 산출물의 섹션 구성, 테이블 형식, 비교 기준 등이 없음.

## 개선 제안

```markdown
## 수정된 실행 계획

### 1-1. 거래소 생태계 (검색 키워드)
- "Binance spot trading fees 2025"
- "OKX API rate limits documentation"
- "Hyperliquid vs dYdX liquidity comparison"
- 각 거래소 공식 fee schedule 페이지 직접 조회

### 1-2. 상품 유형별 특성
- "perpetual futures funding rate mechanism explained"
- "Deribit options liquidity depth"
- "crypto futures liquidation engine comparison"

### 1-3. 규제 환경
- "MiCA crypto regulation 2025 implementation"
- "Singapore MAS crypto licensing requirements"
- "Dubai VARA crypto framework"
- "US SEC crypto enforcement 2025"

### 보고서 구조
1. 거래소 비교표 (CEX vs DEX)
2. 수수료/API 한도 상세표
3. 상품별 특성 비교
4. 규제 환경 요약표
5. 법인 설립 권장 지역 분석
```

## 즉시 필요한 조치
1. 웹 검색 도구를 사용하여 최신 데이터 수집
2. 각 거래소 공식 문서 URL 확인
3. 보고서 템플릿 먼저 작성 후 데이터 채우기