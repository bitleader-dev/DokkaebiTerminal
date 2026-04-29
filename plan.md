# Warp 기반 후속 작업 plan v6 (정리판 — 종료)

> **작성일**: 2026-04-29
> **종료일**: 2026-04-29
> **상태**: ✅ 종료 — Phase F 완료, E/G/H 보류 결정
> **이전 plan**: v5 Phase A/C/D/B 모두 완료 — `notes.md` 보존
> **버전 기준**: `crates/zed/Cargo.toml` v0.4.1

## 진행 결과

| Phase | 결과 | 사유 |
|---|---|---|
| **F** description 편집 모달 markdown 미리보기 | **✅ 완료** | `Markdown::new_with_options + render_mermaid_diagrams: true` + Editor `BufferEdited` subscription 으로 실시간 갱신. 사용자 검증 통과 |
| **E** Text Thread mermaid 렌더 | **보류** | TextThreadEditor 가 buffer 기반 코드 에디터, markdown 렌더 path 없음 — 적용 시 수백 라인 + UX 大 영향 |
| **G** prompt_palette YAML import/export | **보류** | `prompts.json` 자체가 config 디렉터리에 있어 사용자 직접 복사로 동일 효과. ROI 낮음 |
| **H** 활성 터미널 셸 호환성 표시 | **보류** | Terminal entity 에 shell_kind getter 미노출 + 단일 셸 환경 가치 미미. ROI 낮음 |

## 라이선스 게이트 (참고용 보존)

본 plan 적용 시 준수했던 게이트 — 후속 plan 에서도 동일 적용:

1. Warp AGPL 본문 열람 금지. 차단 경로: `D:/Personal Project/Windows/warp-master/**/src/**`. 예외: `*.md`, `Cargo.toml`.
2. 식별자·시그너처·자료구조 카피 금지. 표준 syntax 는 보호 대상 아님.
3. 외부 의존성 추가는 MIT/Apache-2.0/BSD/MPL-2.0 만.
4. AGPL 의존성 금지: `warpdotdev/command-corrections`, `warpdotdev/session-sharing-protocol`, `warpdotdev/warp-proto-apis`.
5. Warp 코드 스니펫 첨부 시 인용 금지.
6. 클린룸 결과 사후 diff 금지.

## Phase F 적용 내역 (완료)

### 변경 파일
| 파일 | 변경 |
|---|---|
| `crates/prompt_palette/Cargo.toml` | `markdown.workspace = true` 1줄 추가 |
| `crates/prompt_palette/src/prompt_form_modal.rs` | `description_preview: Entity<Markdown>` + `_description_subscription: Subscription` 필드, new_create/new_edit 에 동기화, render 에 preview 박스 추가 |
| `assets/locales/ko.json` + `en.json` | `prompt_palette.form.preview_label` 1키 추가 |

### 검증
- ✓ `cargo check -p prompt_palette` 통과 (신규 warning 0)
- ✓ `cargo check -p Dokkaebi` 통과
- ✓ 단위 테스트 28 passed; 0 failed (이전 plan 누적)
- ✓ 사용자 환경 수동 검증 통과 (markdown + mermaid 실시간 갱신)

### 문서 갱신
- ✓ `notes.md` Phase F 항목 추가
- ✓ `release_notes.md` v0.4.1 `### UI/UX 개선` "프롬프트 등록 모달 설명 미리보기" 1줄 추가

## v0.4.1 누적 변경 요약 (이전 plan 포함)

### 새로운 기능
- 프롬프트 팔레트 파라미터 치환 (`{{name}}` placeholder, plan v5 Phase A)

### UI/UX 개선
- AI 응답 mermaid 다이어그램 렌더링 (plan v3 Phase 2)
- 프롬프트 팔레트 태그 다중 분류 (plan v5 Phase C)
- 프롬프트 팔레트 사용 빈도 자동 정렬 (plan v5 Phase D)
- 프롬프트 팔레트 설명 다중 줄 입력 (plan v5 Phase B)
- 프롬프트 등록 모달 설명 미리보기 (plan v6 Phase F)

### 정리
- 공개 배포용 라이선스 사본 동봉 (이전)

### 버그 수정
- 4건 (이전)

## 보류·자동 제외 후보 (별도 plan 시 재진입 가능)

| 후보 | 보류/제외 사유 |
|---|---|
| Text Thread mermaid 렌더 | TextThreadEditor 가 buffer 기반 코드 에디터, markdown 렌더 path 없음. 별도 plan 필요 |
| YAML import/export | `prompts.json` 직접 복사로 동일 효과, ROI 낮음 |
| 활성 셸 호환성 표시 | Terminal entity shell_kind 미노출, 단일 셸 환경 가치 미미 |
| prevent_sleep | 사용자 환경 절전 미사용으로 v3 보류, 환경 변경 시 재진입 |
| Drive (cloud sync) | cloud 비활성 정책 위배 |
| Agent Orchestration | ACP 외부 위임 구조와 충돌 |
| Network Log in-app pane | cloud 비활성으로 호출 빈도 낮음 |
| Managed Secrets (GCP) | cloud 비활성 |
| MCP (warpdotdev fork) | Zed 자체 MCP 보유 |
| Onboarding 슬라이드 | UI 단순화 지향 충돌 |
| Computer Use | 보안 위험 + 일반 IDE 가치 낮음 |
| 블록 기반 터미널 | 수개월 작업, portable-pty 마이그레이션 직후 안정성 위험 |
| Voice Input | 음성 백엔드 라이선스 평가 큰 사전 작업 |

본 plan 종료. 추가 작업 진행 시 새 plan 으로 시작.
