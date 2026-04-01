# 배경화면(Wallpaper) 기능 구현 계획

## 목표
- 설정 화면에 "배경화면" 메뉴를 AI 아래에 추가
- 워크스페이스 중앙 영역(사이드바 제외)에 배경 이미지를 표시
- on/off 토글, 이미지 파일 선택, 맞춤 방식 설정 UI 제공

## 구현 가능성 검증 결과

| 항목 | 결과 | 근거 |
|------|------|------|
| 로컬 파일 이미지 로딩 | ✅ 가능 | `img(PathBuf)` → `Resource::Path(Arc<Path>)` → `fs::read()`. GIF 뷰어, 마크다운 프리뷰 등에서 실제 사용 중 |
| 배경 이미지 레이어링 | ✅ 가능 | GPUI는 자식 렌더링 순서 = z-order. `img().absolute().inset_0()`을 먼저 렌더링하면 콘텐츠 뒤에 배치 |
| 에디터 배경 투명화 | ✅ 가능 | `element.rs:5733` 단일 paint_quad. 이미 `editor_background.a < 0.75` 체크 존재 (line 1773) |
| 터미널 배경 투명화 | ✅ 가능 | `terminal_element.rs:1247` 단일 paint_quad + `terminal_view.rs:1286` 컨테이너 `.bg()` |
| 파일 선택 다이얼로그 | ✅ 가능 | `cx.prompt_for_paths(PathPromptOptions)` — Windows 네이티브 다이얼로그 지원 |
| ObjectFit 매핑 | ✅ 가능 | Contain(맞춤), Cover(채우기), Fill(확대), None(가운데) — GPUI 기본 제공 |
| 설정 페이지 추가 | ✅ 가능 | `page_data.rs:settings_data()`에 함수 추가하는 기존 패턴 |

## 리스크 및 대응

| 리스크 | 영향도 | 대응 |
|--------|--------|------|
| 에디터 거터(gutter) 배경도 불투명 | 중 | `editor_gutter_background`에도 동일 알파 적용 필요 (line 5732) |
| 터미널 컨테이너 배경 별도 존재 | 중 | `terminal_view.rs:1286`의 `.bg()` 호출도 알파 적용 필요 |
| 대용량 이미지(4K+) 성능 | 저 | GPUI 내장 이미지 캐시가 처리. `with_fallback()` 으로 로딩 실패 대응 |
| 이미지 파일 삭제/이동 시 | 저 | `img().with_fallback()` 으로 graceful 처리 |
| 커서 텍스트 반전 색상 | 저 | 이미 `a < 0.75` 분기로 반투명 배경 처리 구현됨 |

## 범위

### 수정 대상 파일
1. `crates/settings_content/src/settings_content.rs` — 배경화면 설정 필드 추가
2. `crates/settings_ui/src/page_data.rs` — 배경화면 설정 페이지 함수 추가 + 메뉴 등록
3. `crates/workspace/src/workspace.rs` — 배경 이미지 렌더링 삽입
4. `crates/editor/src/element.rs` — 에디터 배경 알파 적용 (line 5732-5736)
5. `crates/terminal_view/src/terminal_element.rs` — 터미널 배경 알파 적용 (line 1247)
6. `crates/terminal_view/src/terminal_view.rs` — 터미널 컨테이너 배경 알파 적용 (line 1286)
7. `assets/locales/ko.json` — 한글 문자열
8. `assets/locales/en.json` — 영문 문자열

### 수정하지 않는 것
- 사이드바(dock) 배경 — 불투명 유지
- 테마 시스템 자체 — 변경 없음
- GPUI 프레임워크 — 변경 없음

## 작업 단계

### [x] 1단계: 설정 모델 추가 (`settings_content.rs`)
- `SettingsContent`에 `wallpaper` 필드 추가:
```rust
pub wallpaper: Option<WallpaperSettingsContent>,
```
- `WallpaperSettingsContent` 구조체 정의:
```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct WallpaperSettingsContent {
    /// 배경화면 활성화 여부
    pub enabled: Option<bool>,
    /// 배경 이미지 파일 경로
    pub image_path: Option<String>,
    /// 이미지 맞춤 방식: contain, cover, fill, none
    pub object_fit: Option<WallpaperFitContent>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub enum WallpaperFitContent {
    #[default]
    Cover,
    Contain,
    Fill,
    None,
}
```

### [x] 2단계: 설정 등록 및 읽기
- `crates/workspace/src/`에서 `WallpaperSettings` 등록 (기존 `WorkspaceSettings` 패턴 참고)
- `WallpaperSettings::get_global(cx)` 로 어디서든 읽을 수 있도록 구성

### [x] 3단계: 설정 UI 페이지 추가 (`page_data.rs`)
- `wallpaper_page()` 함수 구현:
  - `SectionHeader("배경화면")` — i18n 키 사용
  - `SettingItem` — 활성화 토글 (`enabled` bool 필드)
  - `SettingItem` — 이미지 경로 (`image_path` String 필드) + 파일 선택 버튼
  - `SettingItem` — 맞춤 방식 (`object_fit` enum 드롭다운: 맞춤/채우기/확대/가운데)
- `settings_data()` 벡터에 `wallpaper_page()` 추가 (ai_page 다음)

### [x] 4단계: 워크스페이스 배경 이미지 렌더링 (`workspace.rs`)
- `impl Render for Workspace`의 `render()` 메서드에서:
  - `WallpaperSettings` 읽기
  - `enabled == true` 이고 `image_path`가 유효하면:
    - `div().id("workspace")` 내부, canvas 자식 다음에 absolute 이미지 추가:
    ```rust
    .when(wallpaper_enabled, |this| {
        this.child(
            img(PathBuf::from(image_path))
                .absolute()
                .inset_0()
                .size_full()
                .object_fit(fit_mode) // 설정값에서 변환
                .with_fallback(|| div().size_0().into_any_element())
        )
    })
    ```

### [x] 5단계: 에디터 배경 투명화 (`element.rs`)
- `paint_background()` 메서드 (line 5728~):
  - `WallpaperSettings::get_global(cx)` 읽기
  - `enabled == true`이면 배경색에 알파 적용:
    ```rust
    let bg = if wallpaper_enabled {
        self.style.background.opacity(0.85)
    } else {
        self.style.background
    };
    window.paint_quad(fill(layout.position_map.text_hitbox.bounds, bg));
    ```
  - 거터 배경도 동일하게 적용 (line 5732)

### [x] 6단계: 터미널 배경 투명화
- `terminal_element.rs` paint 메서드 (line 1247):
  ```rust
  let bg = if wallpaper_enabled {
      layout.background_color.opacity(0.85)
  } else {
      layout.background_color
  };
  window.paint_quad(fill(bounds, bg));
  ```
- `terminal_view.rs` 컨테이너 (line 1286):
  ```rust
  let container_bg = if wallpaper_enabled {
      cx.theme().colors().editor_background.opacity(0.85)
  } else {
      cx.theme().colors().editor_background
  };
  // .bg(container_bg)
  ```

### [x] 7단계: i18n 문자열 추가
- `ko.json`:
  - `"wallpaper.title": "배경화면"`
  - `"wallpaper.enabled": "배경화면 활성화"`
  - `"wallpaper.enabled_description": "워크스페이스 배경에 이미지를 표시합니다."`
  - `"wallpaper.image_path": "이미지 파일"`
  - `"wallpaper.image_path_description": "배경으로 사용할 이미지 파일 경로"`
  - `"wallpaper.object_fit": "맞춤 방식"`
  - `"wallpaper.object_fit_description": "배경 이미지의 크기 조절 방식"`
  - `"wallpaper.fit.cover": "채우기"`
  - `"wallpaper.fit.contain": "맞춤"`
  - `"wallpaper.fit.fill": "확대"`
  - `"wallpaper.fit.none": "가운데"`
- `en.json`: 대응 영문 문자열

### [x] 8단계: 빌드 검증
- `cargo check -p workspace`
- `cargo check -p editor`
- `cargo check -p terminal_view`
- `cargo check -p settings_ui`
- `cargo check -p zed`

## settings.json 예시
```json
{
  "wallpaper": {
    "enabled": true,
    "image_path": "C:\\Users\\jongc\\Pictures\\wallpaper.png",
    "object_fit": "cover"
  }
}
```

## 승인 필요 사항
- [ ] `SettingsContent`에 `wallpaper` 필드 추가 (공개 설정 스키마 변경)
- [ ] 에디터/터미널 paint 로직 수정 (기존 렌더링 동작 변경)
