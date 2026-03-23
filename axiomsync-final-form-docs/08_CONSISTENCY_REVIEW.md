# 08. 정합성 / 이해충돌 검토

이 문서는 셀프 피드백 결과를 기록한다.

## Pass 1 — 이름 충돌 검토

### 문제 후보
- `conversation-native`를 강조하면 Rams를 억지로 conversation으로 넣을 위험
- `run_*`를 중심에 두면 Relay가 왜곡됨

### 수정
- storage core를 `session / entry / artifact / anchor`로 고정
- public semantics는 conversation/episode/insight/procedure 중심 유지

### 판정
- genericity와 conversation usability가 동시에 유지된다

---

## Pass 2 — write ownership 충돌 검토

### 문제 후보
- Relay나 Rams가 kernel DB를 직접 만지면 coupling 증가
- kernel이 Rams state를 canonical로 삼으면 source-of-truth 충돌

### 수정
- single writer discipline을 명시
- Relay/Rams는 raw packet export만 허용

### 판정
- 정본 충돌 없음

---

## Pass 3 — derived memory 과잉 검토

### 문제 후보
- `insight`, `claim`, `procedure`, `verification`를 모두 separate entity로 두면 과해질 수 있음

### 수정
- `claim`을 제거하고 `insight`로 통합
- `verification`은 최소 상태 레코드만 유지

### 판정
- 구조는 충분히 강하면서도 과하지 않다

---

## Pass 4 — query와 ingest 경계 검토

### 문제 후보
- MCP에 write까지 넣으면 boundary가 흐려짐
- HTTP가 query/ingest를 동시에 넓게 담당하면 운영 복잡도 상승

### 수정
- ingest는 library/socket/HTTP
- query는 MCP 중심
- MCP는 read-only

### 판정
- 경계가 선명하다

---

## Pass 5 — evidence 요구 검토

### 문제 후보
- episode/insight/procedure가 evidence 없이 생성되면 품질 하락
- Rams의 check result와 Relay의 selection이 같은 신뢰도로 섞일 수 있음

### 수정
- reusable knowledge에 최소 1개 anchor 요구
- verification을 별도 레코드로 두어 신뢰 수준 분리

### 판정
- evidence-first 원칙 유지

---

## Pass 6 — retrieval 과잉 검토

### 문제 후보
- embeddings / graph / reranker를 core에 넣으면 단순성 저하
- 없으면 장기 검색 품질이 아쉬울 수 있음

### 수정
- FTS를 core, embeddings는 optional
- retrieval index는 disposable로 정의

### 판정
- 본질 유지, 확장 여지 확보

---

## Pass 7 — AxiomRelay / axiomRams 이해충돌 검토

### 문제 후보
- Relay approval과 Rams approval이 같은 시스템으로 합쳐질 위험
- Relay capture state가 Rams run state처럼 보일 위험

### 수정
- capture approval은 Relay
- execution approval은 Rams
- kernel은 approval state의 정본이 아님

### 판정
- 제품 경계 충돌 없음

---

## Pass 8 — 최종 체크리스트

아래 질문에 모두 `예`로 답할 수 있어야 한다.

1. AxiomSync가 단독으로 knowledge kernel 역할을 설명할 수 있는가? — 예
2. AxiomRelay가 kernel modeling 없이도 존재할 수 있는가? — 예
3. axiomRams가 kernel 없이도 run state를 유지할 수 있는가? — 예
4. 세 시스템이 direct DB coupling 없이 연결되는가? — 예
5. reusable memory가 항상 evidence로 거슬러 올라갈 수 있는가? — 예
6. search index를 버리고도 정본을 유지할 수 있는가? — 예
7. conversation use-case와 run use-case를 동시에 수용하는가? — 예

## 최종 판정

이 설계는 다음 세 조건을 동시에 만족한다.

- **단순함**
- **genericity**
- **evidence-backed reuse**

따라서 현재 문맥에서 가장 강한 최종형 문서 세트로 판단한다.
