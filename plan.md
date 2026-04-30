# 터미널 탭 색상 커스터마이징 plan v1 (종료)

> **작성일**: 2026-04-30
> **종료일**: 2026-04-30
> **상태**: ✅ 종료 — Phase 1~5 모두 완료, 사용자 수동 검증 대기
> **버전 기준**: `crates/zed/Cargo.toml` **v0.4.3** (bump 완료)
> **출처**: Warp `warp-master` 분석 결과 1순위 후보 (라이선스 검토 통과)
> **이전 plan**: OSC 133 셸 통합 plan v1 종료 (v0.4.2 출시)
> **사용자 결정 (2026-04-30)** ✅ 모두 적용:
> - 팔레트: **8색 + 없음** (Red / Orange / Yellow / Green / Blue / Purple / Pink / Gray)
> - 표시: **탭 좌측 3px 컬러 바**
> - UI 진입점: **탭 우클릭 메뉴 "탭 색상" 서브메뉴**

## 진행 결과
| Phase | 결과 | 검증 |
|---|---|---|
| 1. 데이터 (enum + 필드) | ✅ 완료 | `cargo check -p terminal_view` |
| 2. 시각 표시 (좌측 컬러 바) | ✅ 완료 | 통과 |
| 3. 우클릭 메뉴 + 액션 | ✅ 완료 | 통과 |
| 4. 영구 저장 (DB 컬럼 + 직렬화) | ✅ 완료 | 통과 |
| 5. i18n + 문서 + 버전 bump | ✅ 완료 | `cargo check -p Dokkaebi` 통과 (신규 warning 0) |

⏳ **사용자 환경 수동 검증 대기**: 우클릭 → 색상 선택 → 좌측 바 표시 / 다른 탭에 다른 색 / 재시작 후 보존 / "없음" 해제

본 plan 종료. 후속 plan 후보로 "블록 단위 navigation (Ctrl+↑/↓)" 가 자연스러운 다음 진입점.

## 목표

사용자가 각 터미널 탭에 색상을 명시 지정해 워크스페이스/원격/태스크/도구별 터미널을 시각적으로 즉시 구분할 수 있게 한다. v0.4.2 의 도구별 아이콘(claude→AiClaude, cargo→FileRust 등) 과 결합되어 다음 사용자 가치 제공:

1. **여러 터미널 탭 식별성 향상** — 동일 도구 다중 터미널(예: claude × 3) 을 색상으로 구분
2. **워크스페이스/원격 컨텍스트 구분** — 로컬·원격·sudo 등 위험도 다른 탭 색으로 표시
3. **장기 사용 탭 라벨링** — 사용자가 의미 있는 탭(예: 빌드용·로그 모니터링용) 을 색으로 마킹

## 비목표 (이번 plan 에서 안 함)

- 탭 색상 자동 부여(워크플로우 컨텍스트 기반) — Warp 자동 부여 정책은 별도 plan
- 탭 그룹화 / 폴더링
- 임의 hex 색상 입력 — 사전 팔레트 8색만 지원(접근성·테마 호환성 보장)
- 색상 단축키 일괄 일괄 변경 — 우클릭 메뉴와 액션만
- DisplayOnly 터미널(스크롤백 전용) 색상 — 일반 PTY 탭만 대상

## 라이선스 게이트

- Warp `warp-master/**/src/**` 본문 열람 금지. 색상 팔레트 컬러 코드는 Dokkaebi 테마 시스템에서 자체 선정.
- 외부 의존성 추가 0건 예상.
- 코드·이름·시그너처 카피 금지.

## 사용자 결정 필요 (3건)

### 1. 색상 팔레트 구성
**기본 제안**: 8색 + "기본"(색상 없음 — 토글 해제용)
- Red / Orange / Yellow / Green / Blue / Purple / Pink / Gray
- GPUI `Color` 시스템과 매핑되는 의미 색상 사용 (테마 라이트/다크 자동 대응)

대안: 6색 또는 12색. **색이 너무 많으면 선택 비용 증가**.

### 2. 색상 표시 방식
**기본 제안**: 탭 좌측 얇은 컬러 바 (3px)
- 탭 컨텐츠(아이콘 + 텍스트) 는 그대로, 좌측에만 색 바 추가
- 도구 아이콘 색조와 분리되어 두 정보 독립 인지

대안:
- 아이콘 색조 변경 (도구 아이콘과 충돌 — 도구 식별 손실)
- 탭 배경 색조 (가독성 우려, 테마 호환 까다로움)
- 좌측 바 + 아이콘 색조 둘 다 (정보 중복)

### 3. UI 진입점
**기본 제안**: 탭 우클릭 메뉴에 "탭 색상" 서브메뉴 (`color1`...`color8` + `없음`)
- rename 과 같은 패턴 — 즉시 적용
- i18n 키 9개 + 색상 9건

대안: 명령 팔레트 액션만, 또는 둘 다.

## 작업 단계

### Phase 1 — 색상 enum + 데이터 구조 (순수 추가)
- [ ] `crates/terminal/src/terminal_settings.rs` 또는 `terminal_view.rs` 에 `TerminalTabColor` enum 추가 (None / Red / Orange / Yellow / Green / Blue / Purple / Pink / Gray, Serialize/Deserialize/JsonSchema). 기본값 None.
- [ ] `crates/terminal_view/src/terminal_view.rs::TerminalView` 에 `custom_color: Option<TerminalTabColor>` 필드 추가 (None = 색상 없음 = 기본).
- [ ] `pub fn set_custom_color(&mut self, color: Option<TerminalTabColor>, cx)` 메서드 — 변경 시 `ItemEvent::UpdateTab` + 영구화 트리거.

### Phase 2 — 시각적 표시 (UI 변경)
- [ ] `tab_content` 의 `h_flex` 시작에 `custom_color` 가 Some 이면 좌측 3px 너비 컬러 바 child 추가. None 이면 기존 레이아웃 유지(픽셀 변화 없음).
- [ ] 색상 → GPUI `Hsla` 매핑 함수 — 각 색상의 라이트/다크 테마 명도/채도 조정. `theme::ActiveTheme` 의 시맨틱 색상 가능한 한 재사용.

### Phase 3 — 우클릭 메뉴 + 액션
- [ ] `crates/terminal_view/src/terminal_view.rs` 에 `actions!(terminal, [SetTabColor { color: TerminalTabColor }])` 액션 등록 — 또는 9개 별도 액션(SetTabColorRed 등).
- [ ] 기존 우클릭 메뉴(deploy_context_menu 또는 동등)에 "탭 색상" 서브메뉴 노드 추가. 9개 항목 + 현재 선택된 색상에 체크 표시.
- [ ] 액션 핸들러 — 선택된 색상으로 `set_custom_color` 호출.

### Phase 4 — 영구 저장 (DB 스키마 — 후방 호환)
- [ ] `persistence.rs::SerializedTerminal` 에 `custom_color: Option<TerminalTabColor>` 추가 (`#[serde(default)]` 으로 후방 호환).
- [ ] serialize/deserialize 코드에서 필드 매핑.
- [ ] **승인 필요**: 영구 저장 스키마 변경. 단 `serde(default)` 로 구버전 데이터는 None 으로 자동 채움 — 사용자 데이터 손실 없음.

### Phase 5 — i18n + 문서 + 검증
- [ ] i18n 키 신규 (ko + en):
  - `terminal.tab.color.menu` ("탭 색상" / "Tab Color")
  - `terminal.tab.color.none` ("없음" / "None")
  - `terminal.tab.color.red` ~ `.gray` (8건)
- [ ] `cargo check -p terminal -p terminal_view -p Dokkaebi` 통과
- [ ] `cargo check --tests -p terminal_view` 통과
- [ ] 사용자 환경 수동 검증: 우클릭 → 색상 선택 → 좌측 바 표시 / 다른 탭에 다른 색 적용 / Dokkaebi 재시작 후 색상 보존 / "없음" 으로 해제
- [ ] `crates/zed/Cargo.toml` v0.4.2 → v0.4.3 bump
- [ ] `assets/release_notes.md` v0.4.3 신규 섹션 — `### 새로운 기능` 또는 `### UI/UX 개선` 1줄
- [ ] `notes.md` Phase 별 변경 기록

## 승인 필요 사항 요약

| 항목 | 사유 |
|---|---|
| **Phase 1** TerminalView 구조체 신규 필드 | 구조 변경 |
| **Phase 3** 액션 enum + 우클릭 메뉴 | 공개 API + UI 변경 |
| **Phase 4** SerializedTerminal 스키마 변경 | DB 스키마 변경 (후방 호환) |
| **버전 bump** | v0.4.3 |
| **사용자 결정 3건** (색 개수·표시 방식·UI 진입점) | 본 plan 상단 참조 |

## 리스크 및 대응

| 리스크 | 대응 |
|---|---|
| 색상이 테마(라이트/다크)에 따라 가독성 저하 | GPUI 시맨틱 색상 사용 + 라이트/다크별 명도 조정 함수 |
| 좌측 컬러 바가 탭 폭 사용량 증가 | 3px 만 추가 — 텍스트 truncate 영향 미미 |
| 우클릭 메뉴 항목 비대화 | 서브메뉴로 묶어 1차 메뉴는 1줄만 |
| 색상 의미 사용자별 다름 | 사전 정의 색상 이름은 중립적(Red/Blue 등). 의미는 사용자 자유 |

## 후속 plan 후보 (본 plan 종료 후)

| 후보 | 트리거 |
|---|---|
| 블록 단위 navigation (Ctrl+↑/↓ 로 이전 명령 점프) | OSC 133 + 색상 안정 동작 후 |
| 탭 색상 자동 부여(워크플로우/원격 등 컨텍스트) | 사용자 사용 패턴 분석 후 |
| Kitty Keyboard Protocol 확장 | modern TUI 필요 사례 발생 시 |

---

**다음 액션**: 위 사용자 결정 3건(① 색 개수·팔레트 ② 표시 방식 ③ UI 진입점) 확정 후 Phase 1 착수.
