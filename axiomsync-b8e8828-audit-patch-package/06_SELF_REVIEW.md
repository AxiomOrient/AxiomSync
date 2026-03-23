# 셀프 리뷰

## Pass 1 — 스키마 과잉 확장 방지

### 발견한 문제
처음엔 README overclaim(`run/task/document`)에 맞춰 스키마를 더 키우는 방향이 떠올랐다.

### 수정
실제 구현이 강한 중심은 conversation + episode + evidence다.  
따라서 이번 패키지는 **스키마 확장보다 surface 축소**를 우선한다.

### 이유
단순함과 본질에 맞다.

---

## Pass 2 — docs chasing 금지

### 발견한 문제
문서와 다르면 문서에 맞춰 구현을 부풀릴 위험이 있었다.

### 수정
문서를 기준으로 평가하지 않고, 실제 구현과 실제 저장 모델을 기준으로 판단했다.

### 이유
사용자 요청과 맞다.

---

## Pass 3 — connector concern 제거 우선

### 발견한 문제
처음에는 canonical noun 정리와 crate split을 같은 레벨로 봤다.

### 수정
우선순위를 다시 조정해 connector runtime leakage 제거를 P0로 올렸다.

### 이유
이게 최종형을 막는 가장 큰 결함이다.

---

## Pass 4 — cursor contract 독립화

### 발견한 문제
source cursor 문제를 작은 구현 디테일로 볼 수 있었다.

### 수정
cursor를 독립 contract로 승격했다.

### 이유
실제 운영 안정성과 external collector 연동성에서 중요도가 높다.

---

## Pass 5 — dead split 방치 금지

### 발견한 문제
workspace split만 존재하고 shipping entrypoint가 old path를 쓰는 상태를 가볍게 볼 수 있었다.

### 수정
이 상태를 구조적 미완료로 명시했다.

### 이유
“있는데 안 쓰는 crate”는 오히려 복잡도만 늘린다.

---

## 최종 판단

현재 구현은 **좋은 커널 코어 + 덜 정리된 release surface**다.  
따라서 최고의 다음 작업은 기능 추가가 아니라 **경계 정리**다.
