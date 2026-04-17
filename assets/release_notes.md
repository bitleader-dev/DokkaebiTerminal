# Dokkaebi 릴리즈 노트

## v0.2.0 (2026-04-17)

### 새로운 기능
- **업스트림 Zed v0.232.2 백포트**: ESLint 3.0.24, LanguageAwareStyling, ACP slash commands 복원, Markdown HTML 정렬, 프로젝트 패널 정렬 순서, SSH nickname 표시, 이미지 멘션 이름, OpenAI reasoning_effort, JSX 컴포넌트 하이라이팅, 카드 레이아웃 패딩, 병합 충돌 상태바 표시기, SVG 폰트 fallback 등 20+ PR 이식
  ([자세한 내용은 공식 Zed 릴리즈 노트](https://github.com/zed-industries/zed/releases/tag/v0.232.2))
- **릴리즈 노트 메뉴**: 설정 메뉴에서 릴리즈 노트를 직접 확인 가능
- **메모장 패널 단축키**: `Ctrl+Shift+M`으로 메모장 패널 토글
- **UI 언어 변경 UI**: 설정 > 일반에서 언어(시스템 언어 / English / 한국어) 선택 가능. 시스템 언어가 기본값이며 OS 언어를 자동 감지하고, 변경 시 재시작 없이 즉시 적용

### UI/UX 개선
- **대규모 i18n 한글화**: 다이얼로그, 툴팁, 버튼, 라벨, 피커 placeholder, 드롭다운, 확장 카드, 키맵 편집기, 상태바 LSP 버튼, ETW 트레이싱 알림 등 100+ 문자열 한글 적용
- **워크스페이스 그룹 패널**: 항목 간 4px 간격 추가로 가독성 향상
- **프로젝트 패널 정렬**: 설정 UI에서 정렬 모드/순서를 직관적인 위치로 이동

### 보안
- **wasmtime 33 → 36 업그레이드**: Dependabot 보안 경고 해결 (critical 2, moderate 10, low 3)

### 정리
- **REPL/Notebook 크레이트 완전 제거**: Windows 환경에서 사용 불가한 Jupyter 관련 코드 정리
- **설정 메뉴 정리**: 불필요한 키맵 파일 메뉴 항목 제거

### 버그 수정
- Non-ASCII 아카이브 스레드 제목 크래시 수정 (한국어 관련)
- 프로젝트 심볼 UTF-8 패닉 수정
- 설정 입력 필드 blur 시 데이터 손실 수정
- Hover popover delay 설정 경로 수정
- 프로필 선택기 Shift+Tab 재렌더링 수정
- Rules library 호버 시 활성 규칙 유지

---

## v0.1.1 (2026-04-15)

### 새로운 기능
- **터미널/셸 UX 개선**: 성능 최적화 및 그룹 패널 간격 조정
- **Claude Code 알림 훅**: 설정 UI 조건 추가 및 컴파일 경고 제거

### 정리
- Dead/미지원 설정 옵션 정리
- i18n 고아 키 정리

---

## v0.1.0 (2026-04-12 ~ 2026-04-14)

### 핵심 변경
- **Zed → Dokkaebi 포크**: Windows 전용 에디터로 재구성
- **portable-pty 마이그레이션**: alacritty EventLoop에서 portable-pty로 터미널 백엔드 교체
- **전체 UI 한글화**: 메뉴, 설정, 액션 이름, 상태 메시지 등 한글 기본 적용
- **라이선스/상류 잔재 정리**: Zed SaaS ToS, cloud 워크플로우, Nix/Docker 빌드 등 제거
- **Windows DPI 처리**: 다중 모니터 환경에서 SetWindowPlacement 좌표계 분리
