# 설정 UI SectionHeader 일괄 i18n 키 전환

## 목표
`crates/settings_ui/src/page_data.rs`의 80개 raw 영문 SectionHeader를 i18n 키로 일괄 전환하여 한국어 UI에서도 섹션 제목이 번역되도록 한다.

## 배경
- `settings_ui.rs:968`이 `t(header, cx)` 호출로 SectionHeader를 i18n 처리함
- 현재 81개 SectionHeader 중 1개(`workspace_panel.title`)만 i18n 키 적용, 나머지 80개는 raw 영문 → 한국어 모드에서도 영문 출력
- 사용자 옵션 B 승인 (2026-04-14)

## 키 네이밍 컨벤션
`settings_page.section.<snake_case>` — 기존 `settings_page.language.semantic_tokens.description` 패턴과 정합

## 작업 단계

### 1단계: 범위 확인
- [x] 코드 조사 완료 (page_data.rs 81개 SectionHeader, 78개 unique 문자열)
- [x] 사용자 승인 — 옵션 B 선택

### 2단계: 수정
- [x] `assets/locales/en.json` — 82개 i18n 키 추가 (영문 = 기존 raw 문자열 그대로)
- [x] `assets/locales/ko.json` — 82개 i18n 키 한글 번역 추가
- [x] `crates/settings_ui/src/page_data.rs` — 84개 SectionHeader 위치 raw 문자열 → i18n 키 교체 (82 unique, Toolbar/Scrollbar 각 2회 중복 포함)

### 3단계: 검증
- [x] `cargo build -p settings_ui` 성공 확인 (14.41s, exit 0, 경고 8건 기존 잔재)

### 4단계: 문서 갱신
- [x] `notes.md` 항목 추가
- [x] `README.md` 수정 안 함 (프로젝트 규칙)

### 5단계: 완료 보고

## 후속 작업 (옵션 A + B-3) 완료 (2026-04-14)

### 옵션 A — `workspace_panel.title` → 신규 컨벤션 통일
- [x] page_data.rs:5256 `workspace_panel.title` → `settings_page.section.workspace_panel`
- [x] ko/en에 `settings_page.section.workspace_panel` 키 추가 + 기존 `workspace_panel.title` 키 삭제
- [x] 빌드 검증 성공 (7.95s)

### 옵션 B-3 — SettingItem title 375개 i18n 키 일괄 전환
- [x] `script/i18n_titles.py` 작성: 329 unique title → 한글 매핑 + snake_case key 자동화
- [x] 키 충돌 검증 (0건)
- [x] en.json/ko.json에 329개 신규 키 일괄 추가 (`settings_page.item.<snake>` 컨벤션)
- [x] page_data.rs의 375개 title 위치 raw 영문 → i18n 키 일괄 교체 (replace 자동화)
- [x] 빌드 검증 성공 (12.19s)
- [x] 잔여 raw title 0건 확인

## 진행 중 — SettingItem `description` i18n 전환 (W4 워크플로우)

### 워크플로우
카테고리(페이지 함수) 분할 + 각 단계마다 자가 검증 + 모호 항목 질문 → 답변 반영 → 적용 → 다음 카테고리

### 카테고리별 description 분포 (총 355개, 354 unique) — 전체 완료
| # | 페이지 함수 | description 수 | 상태 |
|---|---|---|---|
| 1 | keymap_page | 3 | [x] |
| 2 | general_page | 9 | [x] |
| 3 | languages_and_tools_page | 10 | [x] |
| 4 | version_control_page | 14 | [x] |
| 5 | ai_page | 15 | [x] |
| 6 | search_and_files_page | 16 | [x] |
| 7 | terminal_page | 28 | [x] |
| 8 | window_and_layout_page | 35 | [x] |
| 9 | appearance_page | 36 | [x] |
| 10 | panels_page | 57 (+1 누락 추가) | [x] |
| 11 | editor_page | 60 | [x] |
| 12 | wallpaper_page | 72 (+3 wallpaper 섹션 추가) | [x] |
| - | notification_page | 0 (스킵) | [x] |
| - | git_panel.starts_open (한글 raw) | 1 (추가 수정) | [x] |

### 빌드 검증
- `cargo build -p settings_ui` 성공 (6.62s, exit 0)
- 경고 8건 기존 잔재

### 키 충돌 처리
- `environment.program` × 2: 같은 한글 통합 (의미 동일)
- `theme.mode` × 2: 두 번째는 `icon_theme.mode`로 분리
- `toolbar.breadcrumbs` (terminal vs editor): terminal은 `terminal_toolbar`로 분리
- `scrollbar` (terminal vs editor): terminal은 `terminal_scrollbar`로 분리

### 컨벤션
`settings_page.desc.<section>.<item>` (D2 채택)

### 자가 검증 항목
- 번역체 (예: "~을/를 지정합니다" 남발)
- 모호함 (영문 자체가 애매)
- markdown/특수문자 처리
- 도메인 용어 일관성 (vim, lsp, git, multibuffer 등)
- 톤 일관성 (명사형 vs "~합니다" 종결)

### 자동화 스크립트 보관 결정 (2026-04-14)
- `script/i18n_titles.py` (title 329 unique 매핑)
- `script/i18n_descriptions.py` (description 355 ENTRIES + Z3 EN_OVERRIDE)
- **보관 이유**: 향후 번역 수정/개선 작업, 신규 항목 추가 시 재활용 가능
- **재활용 방법**:
  - verify 모드: 키 충돌 / 중복 영문 확인
  - apply 모드: ko/en/page_data.rs 일괄 변경
  - ENTRIES/EN_OVERRIDE에 항목 추가 후 apply 실행

## 후속 정리 완료 (2026-04-14)

### A. ActionLink/SubPageLink description (4건)
- [x] `description: Some("...")` 4건 raw 영문 → i18n 키 전환
- [x] 신규 키: `settings_page.desc.keybindings.edit_keybindings`, `.wallpaper.image_path`, `.agent_configuration.tool_permissions`, `.edit_predictions.configure_providers` (Z3 Zed 제거)

### B. button_text raw 영문 (2건)
- [x] "Open Keymap"/"Browse" → `settings_page.button.open_keymap`/`.browse`

### C. workspace_panel 구 컨벤션 통일 (4곳)
- [x] `workspace_panel.default_width`/`.starts_open` + description → `settings_page.item.*`/`settings_page.desc.workspace_panel.*`

### D. 로케일 키 불일치 정리
- [x] ko.json: 138개 미사용 영문 description 레거시 키 제거
- [x] en.json: 25개 미사용 키 제거 (Collaboration/Debugger/Notification Panel 파생/DAP/menu.run/Breakpoints 등)
- [x] 번역 누락 5개 보정: ko에 "Notification Panel"/"None"/terminal_history.* 2건, en에 "Semantic Tokens", + cleanup 오류로 제거됐던 `None`/terminal_history.* 3개 복원
- [x] 최종: ko/en 키 차이 0건
- [x] `script/i18n_cleanup.py` 신규 (미사용 키 분석 도구, 보관)

### 빌드 검증
- 각 단계별 `cargo build -p settings_ui` 성공

## 다른 Crate i18n 전환 — P2 워크플로우 완료 (2026-04-14)

### P2 (cx 접근 가능한 것만 처리) 누적
| Crate | 완료 | 인프라 추가 |
|---|---|---|
| title_bar | 1/2 | - |
| onboarding (multibuffer_hint) | 3 | - |
| command_palette | 2 | - |
| keymap_editor | 1 | - |
| outline_panel | 2 | Cargo.toml i18n 추가 |
| file_finder | 2 | Cargo.toml i18n 추가 |
| diagnostics | 1/2 | Cargo.toml i18n 추가 (1건 롤백) |
| extensions_ui | 4/6 | - |
| workspace | 1/7 | - |
| **search** | **24건 (21 unique)** | **Cargo.toml i18n 추가** |
| **editor** | **20건 (20 unique)** | - |
| **language_tools** | **22건 (22 unique)** | **Cargo.toml i18n 추가** |
| **누적** | **83건** | **5 crate** |

### 2차분 상세

#### search (Cargo.toml i18n 추가)
- buffer_search.rs: 3건 (Search/Replace with placeholder, Toggle Search Selection tooltip)
- project_search.rs: 18건 (heading 3, label 2, placeholder 4, button 5, tooltip 3, 중복 1)
- search_bar.rs: 1건 (Find in Results)
- search_status_button.rs: 2건 (Project Search × 2)
- **스킵**: render_action_button 시그니처 `&'static str` 제약으로 11건 스킵 (nav 버튼들)

#### editor
- editor.rs: 15건 (Toggle Code Actions, Diff Review 관련, Missing Keybinding Tooltip 5건, Stage/Unstage/Restore/Next/Previous Hunk 5건)
- element.rs: 3건 (Show Symbol Outline, Right-Click to Copy Path, Open File)
- signature_help.rs: 2건 (Previous/Next Signature)
- **스킵**: pending_completion_container/render_relative_row_jump/render_comment_row (cx 없음) 5건

### P2 스킵 (리팩터 필요, 차후 별도 작업)
- title_bar/application_menu.rs:171 "Open Application Menu" Tooltip — PopoverMenu closure 외부 cx 불가
- extensions_ui/extension_version_selector.rs:237 "Incompatible" — 함수 context 미확인
- extensions_ui/components/extension_card.rs:56 "Overridden by dev extension." — 함수 context 미확인
- diagnostics/diagnostics.rs:757 "No problems" — 함수 시그니처에 cx 없음
- search/search_bar.rs render_action_button 11건 — 시그니처 `tooltip: &'static str` 제약 (editor nav/stage/replace 버튼들)
- editor/editor.rs:9724,9844 "Hold"/"Preview" — pending_completion_container(icon) cx 없음
- editor/editor.rs:9927 "Jump to Edit" — render_relative_row_jump cx 없음
- editor/editor.rs:22024,22041 "Cancel"/"Confirm" — render_comment_row cx 없음

## 범위 외 (이번 작업에서 하지 않음)
- 다른 모듈의 i18n 미적용 문자열 (다른 crate들)
- P2 스킵 항목의 리팩터 (상위 함수 시그니처 수정 필요)

---

## 🔁 P2 2차분 완료 (2026-04-14)

### 처리 결과
- search: 24건 (21 unique), Cargo.toml i18n 추가
- editor: 20건 (20 unique)
- language_tools: 22건 (22 unique), Cargo.toml i18n 추가
- **합계**: 66건 추가 (누적 83건, 5 crate에 Cargo.toml 인프라 추가)

### 빌드 검증
- `cargo build -p search -p editor -p language_tools` 성공 (7.22s)

### 키 컨벤션
`<crate>.<feature>` 또는 `<crate>.<feature>.<purpose>` (snake_case)
- 예시: `onboarding.multibuffer_hint.message`, `extensions.view_registry`, `workspace.open_in_default_app`

### P2 워크플로우 규칙 (반드시 준수)
1. **cx 접근 가능**한 문자열만 처리 (Render trait 내부 + render 함수의 `cx: &mut Context<Self>`/`&mut App`)
2. **cx 접근 불가** (closure 외부, 시그니처에 cx 없음 등)는 **스킵** — `plan.md`의 "P2 스킵" 목록에 추가
3. **Cargo.toml 수정 시** `gpui.workspace = true` 다음 줄에 `i18n.workspace = true` 삽입 (알파벳순 유지)
4. **import 추가 시** 파일 상단 첫 `use ...` 다음에 `use i18n::t;` 삽입
5. **문자열 교체**: `"Some String"` → `t("crate.purpose", cx)` (`.into()` 불필요)
6. **ko/en JSON 추가 위치**:
   - en.json: `"Keymap Editor": "Keymap Editor",` 라인 직전
   - ko.json: `"sidebar.open_project": "프로젝트 열기",` 라인 직전
7. **각 crate 완료 후** `cargo build -p <crate>`로 빌드 검증 — 실패 시 즉시 롤백

### 빠른 조사 커맨드
```bash
# crate 내 raw 영문 UI 문자열 스캔
python3 -c "
import re
from pathlib import Path
for rs in Path('crates/<CRATE>').rglob('*.rs'):
    text = rs.read_text(encoding='utf-8')
    for pat, name in [
        (r'Label::new\(\s*\"([A-Z][^\"]{3,})\"\s*\)', 'Label'),
        (r'Tooltip::text\(\s*\"([A-Z][^\"]{3,})\"', 'Tooltip'),
        (r'Button::new\([^,]+,\s*\"([A-Z][^\"]{3,})\"', 'Button'),
        (r'IconButton::new\([^,]+,\s*\"([A-Z][^\"]{3,})\"', 'IconButton'),
    ]:
        for m in re.finditer(pat, text, re.DOTALL):
            ln = text[:m.start()].count(chr(10)) + 1
            print(f'{rs.name}:{ln} [{name}] {m.group(1)[:70]}')
"
```

### JSON 일괄 추가 헬퍼 (ko/en)
```bash
python3 -c "
import json; from pathlib import Path
ADD = [('crate.key1', 'English Text', '한글 번역'), ...]
for p, idx, marker in [
    ('assets/locales/en.json', 1, '  \"Keymap Editor\": \"Keymap Editor\",' + chr(10)),
    ('assets/locales/ko.json', 2, '  \"sidebar.open_project\": \"프로젝트 열기\",' + chr(10)),
]:
    path = Path(p); text = path.read_text(encoding='utf-8'); lines = []
    for k, en, ko in ADD:
        v = en if idx == 1 else ko
        if f'\"{k}\":' not in text:
            lines.append(f'  \"{k}\": {json.dumps(v, ensure_ascii=False)},')
    chunk = chr(10).join(lines) + (chr(10) if lines else '')
    if chunk and marker in text: text = text.replace(marker, chunk + marker, 1)
    path.write_text(text, encoding='utf-8'); print(f'{p}: +{len(lines)}')
"
```

### Zed 브랜드 언급 (Z3 정책) — 발견 시 자동 처리
- 영문 값: Zed 관련 표현을 `the app` 또는 일반화로 교체
- 한글 값: "앱" 또는 자연스럽게 생략
- 이미 처리된 예: `settings_page.desc.workspace_restoration.restore_on_startup`, `disable_git_integration`, `disable_ai` 등

### 톤 (T1 격식체)
- description 계열: "~합니다." 종결
- 단순 label/button: 명사형 OK ("제한 모드", "파일 열기")

### 체크리스트 (각 crate 처리 시)
- [ ] raw 문자열 스캔 + 컨텍스트 확인 (cx 접근 여부)
- [ ] Cargo.toml i18n 추가 (필요 시)
- [ ] use i18n::t; 추가 (필요 시)
- [ ] raw → t("key", cx) 교체
- [ ] ko/en JSON에 키 추가
- [ ] `cargo build -p <crate>` 성공 확인
- [ ] 실패 시 즉시 롤백 (P2 원칙)
- [ ] plan.md 누적 표 + notes.md 갱신

### 완료 후
- plan.md "P2 1차 완료" → "P2 완료 (모든 raw 처리 + 스킵 목록 업데이트)"로 갱신
- notes.md에 2차분 항목 추가
- git commit 제안 (사용자 결정)

## 참고 — Unique 문자열 78종 (영문 → 한글 번역 매핑)
| 영문 | 한글 |
|---|---|
| General Settings | 일반 설정 |
| Workspace Restoration | 워크스페이스 복원 |
| Auto Update | 자동 업데이트 |
| Theme | 테마 |
| Buffer Font | 버퍼 글꼴 |
| UI Font | UI 글꼴 |
| Agent Panel Font | 에이전트 패널 글꼴 |
| Text Rendering | 텍스트 렌더링 |
| Cursor | 커서 |
| Highlighting | 강조 표시 |
| Guides | 가이드 |
| Keybindings | 키 바인딩 |
| Base Keymap | 기본 키맵 |
| Modal Editing | 모달 편집 |
| Auto Save | 자동 저장 |
| Which-key Menu | Which-key 메뉴 |
| Multibuffer | 멀티버퍼 |
| Scrolling | 스크롤 |
| Signature Help | 시그니처 도움말 |
| Hover Popover | 호버 팝업 |
| Drag And Drop Selection | 드래그 앤 드롭 선택 |
| Gutter | 거터 |
| Scrollbar | 스크롤바 |
| Minimap | 미니맵 |
| Toolbar | 툴바 |
| Vim | Vim |
| File Types | 파일 형식 |
| Diagnostics | 진단 |
| Inline Diagnostics | 인라인 진단 |
| LSP Pull Diagnostics | LSP Pull 진단 |
| LSP Highlights | LSP 하이라이트 |
| Languages | 언어 |
| Search | 검색 |
| File Finder | 파일 찾기 |
| File Scan | 파일 스캔 |
| Status Bar | 상태 표시줄 |
| Title Bar | 제목 표시줄 |
| Tab Bar | 탭 표시줄 |
| Tab Settings | 탭 설정 |
| Preview Tabs | 미리보기 탭 |
| Layout | 레이아웃 |
| Window | 창 |
| Pane Modifiers | 페인 보조 키 |
| Pane Split Direction | 페인 분할 방향 |
| Project Panel | 프로젝트 패널 |
| Auto Open Files | 파일 자동 열기 |
| Terminal Panel | 터미널 패널 |
| Outline Panel | 아웃라인 패널 |
| Git Panel | Git 패널 |
| Agent Panel | 에이전트 패널 |
| Notepad Panel | 메모장 패널 |
| Environment | 환경 변수 |
| Font | 글꼴 |
| Display Settings | 표시 설정 |
| Behavior Settings | 동작 설정 |
| Layout Settings | 레이아웃 설정 |
| Advanced Settings | 고급 설정 |
| Git Integration | Git 통합 |
| Git Gutter | Git 거터 |
| Inline Git Blame | 인라인 Git Blame |
| Git Blame View | Git Blame 보기 |
| Branch Picker | 브랜치 선택기 |
| Git Hunks | Git 변경 묶음 |
| General | 일반 |
| Agent Configuration | 에이전트 구성 |
| Context Servers | 컨텍스트 서버 |
| Claude Code | Claude Code |
| Wallpaper | 배경 화면 |
| Indentation | 들여쓰기 |
| Wrapping | 줄바꿈 |
| Indent Guides | 들여쓰기 가이드 |
| Formatting | 포매팅 |
| Autoclose | 자동 닫기 |
| Whitespace | 공백 |
| Completions | 자동 완성 |
| Inlay Hints | 인레이 힌트 |
| Tasks | 작업 |
| Miscellaneous | 기타 |
| LSP | LSP |
| LSP Completions | LSP 자동 완성 |
| Prettier | Prettier |
| Edit Predictions | 편집 예측 |
