# 사이드바 프로젝트 목록: 삭제 버튼 표시 수정 + 등록 목록 영속화

## 배경
이전 작업("우클릭 메뉴 제거 + 우측 삭제 버튼 추가")에서 도입한 사이드바 변경에 두 가지 후속 이슈가 발견됨.

1. **삭제 버튼이 표시되지 않음** — `visible_on_hover("")`로 빈 그룹 이름을 넘겼는데, 부모 행은 `format!("{id_prefix}header-group-{ix}")`라는 이름 있는 group을 사용하므로 hover 매칭이 일어나지 않아 버튼이 항상 invisible 상태가 됨.
2. **앱 재시작 시 등록 목록 휘발** — 현재 `Sidebar::new`는 활성 Workspace의 `root_paths`만 시드로 사용. "단일 활성 worktree" 모델로 전환했기 때문에 재시작 후에는 마지막 활성 프로젝트 1개만 복원되고, 사용자가 사이드바에 등록해 둔 다른 프로젝트는 사라짐.

## 범위
### 수정 대상 (단일 파일)
- `crates/sidebar/src/sidebar.rs`

#### 이슈 1 — 삭제 버튼 hover 매칭 수정
- `render_project_header`:
  - 삭제 `IconButton`의 `.visible_on_hover("")`를 `.visible_on_hover(group_name.clone())`로 변경하여 부모 행 group과 일치시킴.

#### 이슈 2 — 등록 목록 영속화 (KeyValueStore 사용)
- `Sidebar` 구조체:
  - 변경 없음 (기존 `registered_projects: Vec<PathBuf>` 그대로 사용)
- 신규 상수:
  - `const REGISTERED_PROJECTS_KVP_KEY: &str = "sidebar.registered_projects";`
- 신규 헬퍼 메서드:
  - `fn persist_registered_projects(&self, cx: &mut Context<Self>)` — 현재 `registered_projects`를 JSON 문자열로 직렬화한 뒤 `KeyValueStore::global(cx).write_kvp(...)`를 `cx.background_spawn`으로 비동기 호출.
  - `fn load_persisted_registered_projects(cx: &App) -> Vec<PathBuf>` — `KeyValueStore::global(cx).read_kvp(...)`로 저장된 값을 동기 읽고 JSON 역직렬화. 실패/없음 시 빈 Vec 반환.
- `Sidebar::new`:
  - 시드 우선순위 변경: ① KVP에서 로드한 목록이 비어 있지 않으면 그것을 사용, ② 비어 있으면 기존 로직(workspace root_paths)으로 폴백.
  - 시드 후 활성 workspace의 root_path도 등록 목록에 포함되어 있는지 확인하여 없으면 추가(세션 복원이 KVP보다 다른 프로젝트를 열었을 가능성 대비).
  - **중요**: KVP에서 복원된 프로젝트 중 활성 항목이 아닌 것들은 단순히 등록 목록에만 추가됨. workspace에 worktree를 자동으로 다시 열지는 않음(현재 모델은 "단일 활성 worktree"이므로 복수 worktree 동시 로딩은 모델 위반).
- 등록 목록 변경 지점에서 영속화 호출:
  - `add_project_folder`: `registered_projects.push` 직후 `persist_registered_projects` 호출.
  - `remove_registered_project`: `registered_projects.retain` 직후 `persist_registered_projects` 호출.

### 손대지 않는 것
- `switch_active_project` 동작 (영속화는 등록 목록 변경 시에만, 활성 전환 시에는 X)
- `Workspace`/`MultiWorkspace` 직렬화
- DB 스키마 (KVP 테이블은 이미 존재)
- 다른 패널/모듈

## 작업 단계
- [x] 1. plan.md 작성 및 승인 대기
- [x] 2. 이슈 1: `visible_on_hover` group 이름 매칭 수정
- [x] 3. 이슈 2: KVP 키 상수 + load/persist 헬퍼 메서드 추가
- [x] 4. 이슈 2: `Sidebar::new`에서 KVP 우선 시드 + 활성 root 보강
- [x] 5. 이슈 2: `add_project_folder` / `remove_registered_project`에 영속화 훅 추가
- [x] 6. 빌드 검증: `cargo build -p sidebar`, `cargo build`
- [x] 7. notes.md 갱신
- [x] 8. 완료 보고

## 검증 방법
- 빌드: `cargo build -p sidebar`, `cargo build`
- 수동:
  - **이슈 1**: 프로젝트 행에 마우스를 올렸을 때 우측 끝에 휴지통 아이콘 버튼이 노출되는지 확인. 마우스 떠나면 사라지는지 확인.
  - **이슈 2**: 프로젝트 2개 이상 등록 → 앱 종료 → 재실행 후 사이드바에 등록했던 프로젝트들이 모두 표시되는지 확인. 활성 프로젝트는 마지막에 선택했던 것이 그대로 활성 상태인지 확인.
  - **이슈 2 회귀**: 삭제 버튼으로 항목 제거 후 재시작 시 제거된 항목이 복구되지 않는지 확인.

## 승인 필요 항목
- **이슈 2 영속화 메커니즘 채택**: `KeyValueStore::global(cx)` 사용 (이미 zed 전반에서 쓰이는 기존 KV 저장소, 스키마 변경 없음, 의존성 추가 없음). 단, 사이드바에서 해당 모듈을 처음 직접 호출하게 됨 → 새 import 1줄 추가됨. 다른 영속화 방법(workspace serialization 확장 등)을 선호하시면 알려주세요.
- **복원 의미 정의**: 본 계획은 "등록 목록의 모든 프로젝트가 사이드바 행으로 표시됨, 단 활성 worktree는 마지막 1개만 자동 로드"임. 사용자가 클릭하면 그때 worktree로 전환됨. "재시작 시 모든 등록 프로젝트의 worktree를 다 열고 싶다"는 의미가 아니면 이 정의가 맞는지 확인 필요.
