# 프롬프트 팔레트 기능 구현 계획

## 목표
- 터미널에서 단축키로 호출하는 Command Palette 스타일 프롬프트 선택 팝업 구현
- 프롬프트 선택 시 활성 터미널에 해당 프롬프트 텍스트 입력
- 프롬프트 등록/편집/삭제 UI 제공 (프롬프트 + 설명글 + 카테고리)
- 목록 항목: 프롬프트 텍스트(주) + 설명글(보조, 작은 글씨)

## 구현 가능성 검증 결과

| 항목 | 결과 | 근거 |
|------|------|------|
| Picker 기반 팝업 | ✅ 가능 | `command_palette`, `theme_selector`, `locale_selector` 등 동일 패턴 다수 존재 |
| 터미널 전용 단축키 | ✅ 가능 | `default-windows.json`에서 `"context": "Terminal"` 바인딩 패턴 이미 사용 중 |
| 2줄 렌더링 (제목+설명) | ✅ 가능 | `branch_picker`에서 `v_flex().child(title).child(description)` 패턴 사용 중 |
| 터미널 텍스트 입력 | ✅ 가능 | `terminal::SendText(String)` 또는 `TerminalView`의 입력 메서드 활용 |
| 사용자 데이터 JSON 저장 | ✅ 가능 | `settings.json` 또는 별도 JSON 파일 저장/로드 패턴 존재 |
| 카테고리 필터링 | ✅ 가능 | Picker의 `render_header()` 또는 섹션 구분 렌더링으로 구현 가능 |
| 입력 폼 모달 | ✅ 가능 | `commit_modal`, `add_llm_provider_modal` 등 InputField 기반 폼 패턴 존재 |

## 리스크 및 대응

| 리스크 | 영향도 | 대응 |
|--------|--------|------|
| 프롬프트 데이터 파일 손상 | 중 | JSON 파싱 실패 시 빈 목록으로 폴백, 백업 로직 고려 |
| 카테고리 필터 + 퍼지 검색 혼합 | 저 | Picker delegate 내부에서 카테고리 프리필터 후 퍼지 매칭 |
| 터미널에 멀티라인 프롬프트 입력 | 저 | `\n`을 포함한 텍스트 전송 시 터미널 동작 확인 필요 |

## 범위

### 새로 생성하는 파일
1. `crates/prompt_palette/Cargo.toml` — 크레이트 정의
2. `crates/prompt_palette/src/lib.rs` — 모듈 엔트리
3. `crates/prompt_palette/src/prompt_palette.rs` — 팔레트 팝업 (Picker 기반)
4. `crates/prompt_palette/src/prompt_store.rs` — 데이터 모델 + JSON 저장/로드
5. `crates/prompt_palette/src/prompt_form_modal.rs` — 등록/편집 모달

### 수정하는 파일
1. `Cargo.toml` (루트) — workspace members에 `prompt_palette` 추가
2. `crates/zed/Cargo.toml` — `prompt_palette` 의존성 추가
3. `crates/zed/src/main.rs` — `prompt_palette::init(cx)` 호출
4. `assets/keymaps/default-windows.json` — 터미널 컨텍스트에 단축키 추가
5. `assets/keymaps/default-macos.json` — macOS 단축키 추가
6. `assets/keymaps/default-linux.json` — Linux 단축키 추가
7. `assets/locales/ko.json` — 한글 문자열
8. `assets/locales/en.json` — 영문 문자열

### 수정하지 않는 것
- 기존 `command_palette` 크레이트 — 변경 없음
- `picker` 크레이트 — 변경 없음 (그대로 사용)
- `terminal_view` 크레이트 — 변경 없음 (액션만 dispatch)
- 설정 시스템 자체 — 변경 없음

## 데이터 모델

```rust
/// 프롬프트 항목
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptEntry {
    /// 고유 ID (UUID)
    pub id: String,
    /// 터미널에 입력될 프롬프트 텍스트
    pub prompt: String,
    /// 목록에 표시될 설명글
    pub description: String,
    /// 카테고리 (예: "git", "docker", "일반")
    pub category: String,
    /// 생성 시각
    pub created_at: String,
}

/// 전체 프롬프트 저장 구조
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PromptCollection {
    pub prompts: Vec<PromptEntry>,
}
```

### 저장 위치
- `~/.config/dokkaebi/prompts.json` (또는 플랫폼별 config 디렉토리)

## 작업 단계

### [x] 1단계: 크레이트 생성 및 데이터 모델
- `crates/prompt_palette/` 디렉토리 및 `Cargo.toml` 생성
- `prompt_store.rs`: `PromptEntry`, `PromptCollection` 구조체 정의
- JSON 파일 로드/저장 함수 (`load_prompts`, `save_prompts`)
- 루트 `Cargo.toml`의 members에 추가

### [x] 2단계: 프롬프트 팔레트 팝업 (Picker 기반)
- `prompt_palette.rs`: `PromptPalette` 구조체 + `ModalView` 구현
- `PromptPaletteDelegate` + `PickerDelegate` 구현:
  - `render_match()`: 2줄 렌더링
    - 1줄: 프롬프트 텍스트 (HighlightedLabel, 퍼지 매칭 하이라이트)
    - 2줄: 설명글 (작은 회색 텍스트) + 카테고리 뱃지
  - `confirm()`: 선택한 프롬프트를 활성 터미널에 `SendText`로 전송
  - `dismissed()`: 팝업 닫기
- 카테고리 필터: 검색창에 `@카테고리명 ` 접두사로 필터링 또는 별도 필터 UI
- 퍼지 검색: `fuzzy` 크레이트 활용 (프롬프트 텍스트 + 설명글 대상)

### [x] 3단계: 프롬프트 등록/편집 모달
- `prompt_form_modal.rs`: `PromptFormModal` 구조체 + `ModalView` 구현
- 입력 필드 3개:
  - 프롬프트 텍스트 (멀티라인 Editor)
  - 설명글 (단일행 InputField)
  - 카테고리 (단일행 InputField 또는 드롭다운)
- 하단 버튼: 저장 / 취소 (편집 모드일 때는 삭제 버튼 추가)
- 저장 시 `PromptStore`에 추가/갱신 → JSON 파일 기록

### [x] 4단계: 팔레트에서 편집/삭제 진입점
- 팔레트 목록 항목에 편집 버튼 (연필 아이콘) 또는 컨텍스트 메뉴
- 팔레트 하단에 "새 프롬프트 등록" 버튼
- 삭제: 편집 모달 내 삭제 버튼 → 확인 다이얼로그 후 삭제

### [x] 5단계: 단축키 및 액션 등록
- 액션 정의:
  - `prompt_palette::Toggle` — 팔레트 열기/닫기
  - `prompt_palette::NewPrompt` — 등록 모달 열기
- `init(cx)` 함수에서 워크스페이스에 액션 등록
- 키맵 파일에 터미널 컨텍스트 바인딩 추가:
  ```json
  {
      "context": "Terminal",
      "bindings": {
          "ctrl-shift-p": "prompt_palette::Toggle"
      }
  }
  ```
  (ctrl-shift-p는 예시, 기존 바인딩과 충돌하지 않는 키 선택 필요)

### [x] 6단계: zed 크레이트 통합
- `crates/zed/Cargo.toml`에 `prompt_palette` 의존성 추가
- `crates/zed/src/main.rs`에서 `prompt_palette::init(cx)` 호출

### [x] 7단계: i18n 문자열 추가
- `ko.json`:
  - `"prompt_palette.title": "프롬프트 팔레트"`
  - `"prompt_palette.search_placeholder": "프롬프트 검색..."`
  - `"prompt_palette.new_prompt": "새 프롬프트"`
  - `"prompt_palette.edit_prompt": "프롬프트 편집"`
  - `"prompt_palette.delete_prompt": "프롬프트 삭제"`
  - `"prompt_palette.delete_confirm": "이 프롬프트를 삭제하시겠습니까?"`
  - `"prompt_palette.prompt_text": "프롬프트"`
  - `"prompt_palette.description": "설명"`
  - `"prompt_palette.category": "카테고리"`
  - `"prompt_palette.save": "저장"`
  - `"prompt_palette.cancel": "취소"`
  - `"prompt_palette.no_prompts": "등록된 프롬프트가 없습니다"`
- `en.json`: 대응 영문 문자열

### [x] 8단계: 빌드 검증
- `cargo check -p prompt_palette`
- `cargo check -p zed`
- 기능 동작 확인:
  - 터미널 탭에서 단축키 → 팔레트 표시
  - 비터미널 탭에서 단축키 → 반응 없음
  - 프롬프트 선택 → 터미널에 텍스트 입력
  - 등록/편집/삭제 정상 동작
  - 카테고리 필터링 및 퍼지 검색 동작

## UI 와이어프레임

### 프롬프트 팔레트 (선택 팝업)
```
┌─────────────────────────────────────┐
│ 🔍 프롬프트 검색...                   │
├─────────────────────────────────────┤
│ ▶ docker ps -a --format "..."      │  ← 선택 상태 (하이라이트)
│   실행 중인 컨테이너 목록 조회  [docker] │  ← 설명글(작은 글씨) + 카테고리 뱃지
├─────────────────────────────────────┤
│   git log --oneline -20             │
│   최근 20개 커밋 이력 확인     [git]   │
├─────────────────────────────────────┤
│   kubectl get pods -n production    │
│   운영 환경 Pod 상태 확인  [k8s]      │
├─────────────────────────────────────┤
│                    [+ 새 프롬프트]    │
└─────────────────────────────────────┘
```

### 프롬프트 등록/편집 모달
```
┌─────────────────────────────────────┐
│  프롬프트 등록 (또는 편집)             │
├─────────────────────────────────────┤
│ 프롬프트:                            │
│ ┌─────────────────────────────────┐ │
│ │ docker ps -a --format "..."     │ │
│ └─────────────────────────────────┘ │
│ 설명:                               │
│ ┌─────────────────────────────────┐ │
│ │ 실행 중인 컨테이너 목록 조회       │ │
│ └─────────────────────────────────┘ │
│ 카테고리:                            │
│ ┌─────────────────────────────────┐ │
│ │ docker                          │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│         [삭제]    [취소]    [저장]    │
└─────────────────────────────────────┘
```

## 단축키 후보
- `ctrl-shift-;` (Windows/Linux) / `cmd-shift-;` (macOS)
- 기존 바인딩 충돌 여부 최종 확인 후 결정

## 승인 필요 사항
- [ ] 새 크레이트 `prompt_palette` 추가 (워크스페이스 구조 변경)
- [ ] `crates/zed/Cargo.toml` 의존성 추가
- [ ] 키맵 파일 수정 (단축키 추가)
