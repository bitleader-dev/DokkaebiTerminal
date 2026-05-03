# taffy 0.9.0 → 0.10.1 업그레이드 plan v1

> **작성일**: 2026-05-03
> **상태**: ✅ 종료 (2026-05-03)
> **트리거**: GitHub dependabot MEDIUM 경고 (grid 0.18.0, GHSA-38c5-483c-4qqp Integer Overflow UB) → 사용자 옵션 1 (taffy 0.10.1) 선택, 추가 질문 "원본 소스가 0.10.1 이면 원본 대로" 에 따라 upstream 동기화 경로 확정

## 목표
`crates/gpui/Cargo.toml` 의 `taffy = "=0.9.0"` 을 상류 Zed 와 동일한 `taffy = "=0.10.1"` 로 업그레이드. 부수 효과로 의존성 트리의 `grid 0.18.0 → 1.0.x` 자동 전환되어 dependabot MEDIUM 경고 해소.

## 사전 분석 (확인 완료)
- 상류 Zed 의 `crates/gpui/Cargo.toml` 도 `taffy = "=0.10.1"` 사용 중 (grep 으로 확인)
- taffy 0.10.0 changelog 의 breaking changes:
  1. `LayoutPartialTree::set_*` API 가 `&LayoutInput` 받도록 변경 — **gpui 는 `LayoutPartialTree` 직접 구현 없음** (`grep "LayoutPartialTree\|impl.*for.*TaffyLayoutEngine" crates/gpui/src/taffy.rs` 0건). 영향 없음
  2. `DetailedGridTracksInfo` public 모듈로 이동 — gpui 사용 없음
  3. `TaffyTree::write_tree` 신규 — 추가, breaking 아님
  4. MSRV 1.71 — Dokkaebi 의 rustc 버전이 더 높음
- taffy 0.10.1 (0.10.0 + auto-repeat fix) 단일 patch
- gpui 의 taffy API 사용처: `TaffyTree::new`/`enable_rounding`/`new_leaf`/`new_leaf_with_context`/`compute_layout_with_measure`/`TraversePartialTree`/`style_helpers::{fr,length,minmax,repeat,max_content,min_content}`/`GridTemplateComponent` — 모두 high-level. 0.10 에서 시그너처 그대로 유지될 가능성 매우 높음

## 범위 (코드 변경)

### 1. 의존성 버전 bump
- `crates/gpui/Cargo.toml` 한 줄: `taffy = "=0.9.0"` → `taffy = "=0.10.1"`
- 이외 Cargo.toml 변경 없음 (workspace deps 에 taffy 없음 확인 완료)

### 2. Cargo.lock 갱신
- `cargo update -p taffy` — taffy 0.9.0 → 0.10.1, grid 0.18.0 → 1.0.x 자동 적용

### 3. 코드 호환성 fix (예측 0~2 사이트)
- 빌드 실패 시 에러별로 minimal patch
- 예상 영향 위치 (예측이고 실제 확인 후 결정):
  - `crates/gpui/src/taffy.rs:8-14` import 블록의 path 변경 가능성
  - `GridTemplateComponent` 의 generic 파라미터 변경 (0.9.0 changelog 에 "Style generic over CheapCloneStr" 명시)

## 작업 단계 (순서 준수)

1. **[x]** `crates/gpui/Cargo.toml` `taffy = "=0.9.0"` → `=0.10.1` 수정
2. **[x]** `cargo update -p taffy` 실행, Cargo.lock 의 grid 1.0.x 확인
3. **[x]** `cargo check -p gpui --tests` 1차 검증 (gpui 단일 크레이트 빠른 검증)
4. **[x]** 에러 발생 시 minimal fix, 0건이면 skip
5. **[x]** `cargo check -p Dokkaebi --tests` 최종 검증 (전체 빌드 + 테스트)
6. **[x]** `notes.md` 갱신 — taffy 0.10.1 백포트 + grid CVE 해소 기록
7. **[x]** `assets/release_notes.md` `### 보안` 카테고리 항목 보강 또는 별도 업데이트
8. **[x]** 커밋 + push

## 검증 방법
- `cargo check -p gpui --tests` — 신규 warning/error 0건
- `cargo check -p Dokkaebi --tests` — 신규 warning/error 0건
- Cargo.lock 의 `grid` 버전이 1.0.x 인지 확인
- 사용자 환경 수동 검증 권장:
  - (a) Dokkaebi 실행 → UI 렌더 정상
  - (b) grid layout 사용 컴포넌트 (예: Setting UI 의 PluginAction 그리드) 정상 표시
  - (c) flexbox 레이아웃 정상 (대부분의 패널)

## 승인 필요 항목
1. **본 plan 자체 승인** — `feedback_plan_approval.md` 룰
2. **의존성 버전 변경 동의** — taffy 0.9.0 → 0.10.1 (CLAUDE.md "의존성 추가/버전 변경" 룰)
3. **API 충돌 시 대응 권한** — gpui 의 taffy 사용 코드 minimal 수정 (예상 0~2 사이트, 만약 더 많으면 plan 재작성)

## 리스크 및 대응
- **리스크 1**: 빌드 실패 (3+ 사이트 충돌) — minimal fix 가 실효성 없으면 plan 중단, 0.9.2 폴백 옵션 제안
- **리스크 2**: 빌드 통과하지만 런타임 레이아웃 회귀 — 사용자 수동 검증으로 catch. 회귀 발견 시 git revert 로 즉시 롤백 (단일 PR 단위)
- **리스크 3**: Dokkaebi 가 미백포트한 상류 gpui PR (Pixel snapping #54728 등) 과의 간접 충돌 — taffy.rs 의 diff 가 큰 상태이지만, 본 plan 은 taffy API 직접 호출처만 건드리고 Pixel snapping 같은 상류 미백포트 영역은 손대지 않음
- **롤백**: 단일 Cargo.toml 변경 + Cargo.lock 갱신 → `git checkout HEAD -- crates/gpui/Cargo.toml Cargo.lock` 으로 즉시 복구

## 비범위 (본 plan 에서 다루지 않음)
- 상류 Zed 의 Pixel snapping (#54728) 기타 gpui 리팩터 백포트 — 별도 plan 후보
- taffy 0.10 의 신기능 (direction RTL, float, parse) 활용 — 사용자 가치 명확하지 않음
- 다른 dependabot 경고 처리
