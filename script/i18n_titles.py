"""
SettingItem title을 i18n 키로 일괄 전환하기 위한 매핑 + 출력 생성기.
- 입력: 영문 title → 한글 번역 매핑
- 출력: ko/en JSON 청크 + page_data.rs 교체 매핑
"""
import re
import sys

# 영문 title → 한글 번역 매핑 (329개 unique)
MAPPING = {
    "AI": "AI",
    "Activate On Close": "닫을 때 활성화",
    "Active Encoding Button": "현재 인코딩 버튼",
    "Active File Name": "현재 파일 이름",
    "Active Language Button": "현재 언어 버튼",
    "Active Line Width": "현재 라인 너비",
    "Agent Panel Button": "에이전트 패널 버튼",
    "Agent Panel Default Height": "에이전트 패널 기본 높이",
    "Agent Panel Default Width": "에이전트 패널 기본 너비",
    "Agent Panel Dock": "에이전트 패널 위치",
    "Agent Review": "에이전트 검토",
    "Allow Rewrap": "재줄바꿈 허용",
    "Allowed": "허용",
    "Alternate Scroll": "대체 스크롤",
    "Always Treat Brackets As Autoclosed": "항상 괄호를 자동 닫기로 처리",
    "Appearance": "외관",
    "Arguments": "인자",
    "Auto Fold Directories": "디렉토리 자동 접기",
    "Auto Indent On Paste": "붙여넣기 시 자동 들여쓰기",
    "Auto Indent": "자동 들여쓰기",
    "Auto Replace Emoji Shortcode": "이모지 단축코드 자동 변환",
    "Auto Reveal Entries": "항목 자동 노출",
    "Auto Save Mode": "자동 저장 모드",
    "Auto Signature Help": "자동 시그니처 도움말",
    "Auto Update": "자동 업데이트",
    "Autoscroll On Clicks": "클릭 시 자동 스크롤",
    "Background Coloring": "배경 색상",
    "Background Opacity": "배경 투명도",
    "Base Keymap": "기본 키맵",
    "Bold Folder Labels": "폴더 라벨 굵게",
    "Border Size": "테두리 크기",
    "Bottom Dock Layout": "하단 도크 레이아웃",
    "Breadcrumbs": "경로 표시",
    "Buffer Font Size": "버퍼 글꼴 크기",
    "Cancel Generation On Terminal Stop": "터미널 중지 시 생성 취소",
    "Case Sensitive": "대소문자 구분",
    "Center on Match": "일치 항목 중앙 정렬",
    "Centered Layout Left Padding": "중앙 레이아웃 왼쪽 여백",
    "Centered Layout Right Padding": "중앙 레이아웃 오른쪽 여백",
    "Claude Code Task Completion Bell": "Claude Code 작업 완료 알림음",
    "Close on File Delete": "파일 삭제 시 닫기",
    "Code Actions On Format": "포맷 시 코드 액션",
    "Code Actions": "코드 액션",
    "Collapse Untracked Diff": "추적되지 않은 Diff 접기",
    "Coloring": "색상",
    "Colorize Brackets": "괄호 색상화",
    "Completion Detail Alignment": "자동 완성 상세 정렬",
    "Completion Menu Scrollbar": "자동 완성 메뉴 스크롤바",
    "Configure Providers": "제공자 구성",
    "Context Server Timeout": "컨텍스트 서버 타임아웃",
    "Copy On Select": "선택 시 복사",
    "Current Line Highlight": "현재 라인 강조",
    "Cursor Blink": "커서 깜빡임",
    "Cursor Blinking": "커서 깜빡임 사용",
    "Cursor Position Button": "커서 위치 버튼",
    "Cursor Shape - Insert Mode": "커서 모양 - Insert 모드",
    "Cursor Shape - Normal Mode": "커서 모양 - Normal 모드",
    "Cursor Shape - Replace Mode": "커서 모양 - Replace 모드",
    "Cursor Shape - Visual Mode": "커서 모양 - Visual 모드",
    "Cursor Shape": "커서 모양",
    "Cursors": "커서",
    "Custom Digraphs": "사용자 디그래프",
    "Custom Line Height": "사용자 라인 높이",
    "Dark Icon Theme": "어두운 아이콘 테마",
    "Dark Theme": "어두운 테마",
    "Debounce": "디바운스",
    "Default Height": "기본 높이",
    "Default Mode": "기본 모드",
    "Default Width": "기본 너비",
    "Delay (milliseconds)": "지연 (밀리초)",
    "Delay": "지연",
    "Detect Virtual Environment": "가상 환경 감지",
    "Diagnostic Badges": "진단 배지",
    "Diagnostics Button": "진단 버튼",
    "Diagnostics": "진단",
    "Diff Stats": "Diff 통계",
    "Diff View Style": "Diff 보기 스타일",
    "Directory": "디렉토리",
    "Disable AI": "AI 비활성화",
    "Disable Git Integration": "Git 통합 비활성화",
    "Disable in Language Scopes": "언어 범위에서 비활성화",
    "Display In Text Threads": "텍스트 스레드에 표시",
    "Display In": "표시 위치",
    "Display Mode": "표시 모드",
    "Double Click In Multibuffer": "멀티버퍼에서 더블 클릭",
    "Drag and Drop": "드래그 앤 드롭",
    "Drop Size Target": "드롭 크기 대상",
    "Edit Debounce Ms": "편집 디바운스 (ms)",
    "Edit Keybindings": "키 바인딩 편집",
    "Editor": "편집기",
    "Enable Feedback": "피드백 사용",
    "Enable Git Diff": "Git Diff 사용",
    "Enable Git Status": "Git 상태 사용",
    "Enable Keep Preview On Code Navigation": "코드 탐색 시 미리보기 유지",
    "Enable Language Server": "언어 서버 사용",
    "Enable Preview File From Code Navigation": "코드 탐색에서 미리보기 파일 사용",
    "Enable Preview From File Finder": "파일 찾기에서 미리보기 사용",
    "Enable Preview From Multibuffer": "멀티버퍼에서 미리보기 사용",
    "Enable Preview From Project Panel": "프로젝트 패널에서 미리보기 사용",
    "Enable Preview Multibuffer From Code Navigation": "코드 탐색에서 미리보기 멀티버퍼 사용",
    "Enable Wallpaper": "배경 화면 사용",
    "Enabled": "사용",
    "Ensure Final Newline On Save": "저장 시 마지막 줄바꿈 보장",
    "Entry Spacing": "항목 간격",
    "Environment Variables": "환경 변수",
    "Excerpt Context Lines": "발췌 컨텍스트 줄",
    "Expand Edit Card": "편집 카드 확장",
    "Expand Excerpt Lines": "발췌 줄 확장",
    "Expand Outlines With Depth": "개요 깊이 확장",
    "Expand Terminal Card": "터미널 카드 확장",
    "Extend Comment On Newline": "줄바꿈 시 주석 연장",
    "Fallback Branch Name": "대체 브랜치 이름",
    "Fast Scroll Sensitivity": "빠른 스크롤 감도",
    "Fetch Timeout (milliseconds)": "가져오기 타임아웃 (밀리초)",
    "File Icons": "파일 아이콘",
    "File Scan Exclusions": "파일 스캔 제외",
    "File Scan Inclusions": "파일 스캔 포함",
    "File Type Associations": "파일 형식 연결",
    "Folder Icons": "폴더 아이콘",
    "Font Fallbacks": "글꼴 대체",
    "Font Family": "글꼴",
    "Font Features": "글꼴 기능",
    "Font Size": "글꼴 크기",
    "Font Weight": "글꼴 굵기",
    "Format On Save": "저장 시 포매팅",
    "Formatter": "포매터",
    "General": "일반",
    "Git Diff": "Git Diff",
    "Git Panel Button": "Git 패널 버튼",
    "Git Panel Default Width": "Git 패널 기본 너비",
    "Git Panel Dock": "Git 패널 위치",
    "Git Panel Status Style": "Git 패널 상태 스타일",
    "Git Status Indicator": "Git 상태 표시기",
    "Git Status": "Git 상태",
    "Global Substitution Default": "전역 치환 기본값",
    "Go To Definition Fallback": "정의로 이동 대체",
    "Hard Tabs": "하드 탭",
    "Helix Mode": "Helix 모드",
    "Hidden Files": "숨김 파일",
    "Hide .gitignore": ".gitignore 숨기기",
    "Hide Hidden": "숨김 항목 숨기기",
    "Hide Mouse": "마우스 숨기기",
    "Hide Root": "루트 숨기기",
    "Highlight on Yank Duration": "Yank 시 강조 시간",
    "Horizontal Scroll Margin": "가로 스크롤 여백",
    "Horizontal Scroll": "가로 스크롤",
    "Horizontal Scrollbar": "가로 스크롤바",
    "Horizontal Split Direction": "가로 분할 방향",
    "Hunk Style": "Hunk 스타일",
    "Icon Theme Name": "아이콘 테마 이름",
    "Icon Theme": "아이콘 테마",
    "Image Path": "이미지 경로",
    "Image Viewer": "이미지 뷰어",
    "Inactive Opacity": "비활성 투명도",
    "Include Ignored in Search": "검색에 무시된 항목 포함",
    "Include Ignored": "무시된 항목 포함",
    "Include Warnings": "경고 포함",
    "Indent Size": "들여쓰기 크기",
    "Inline Code Actions": "인라인 코드 액션",
    "Insert Mode": "삽입 모드",
    "JSX Tag Auto Close": "JSX 태그 자동 닫기",
    "Keep Selection On Copy": "복사 시 선택 유지",
    "Keymap": "키맵",
    "LSP Document Colors": "LSP 문서 색상",
    "LSP Document Symbols": "LSP 문서 심볼",
    "LSP Folding Ranges": "LSP 접기 범위",
    "Language Servers": "언어 서버",
    "Languages & Tools": "언어 및 도구",
    "Light Icon Theme": "밝은 아이콘 테마",
    "Light Theme": "밝은 테마",
    "Line Height": "라인 높이",
    "Line Width": "라인 너비",
    "Linked Edits": "연결된 편집",
    "Max Scroll History Lines": "최대 스크롤 기록 줄 수",
    "Max Severity": "최대 심각도",
    "Max Width Columns": "최대 너비 컬럼",
    "Maximum Tabs": "최대 탭 수",
    "Menu Delay": "메뉴 지연",
    "Message Editor Min Lines": "메시지 편집기 최소 줄 수",
    "Middle Click Paste": "중간 클릭 붙여넣기",
    "Min Line Number Digits": "최소 줄 번호 자릿수",
    "Minimum Column": "최소 컬럼",
    "Minimum Contrast For Highlights": "강조 최소 대비",
    "Minimum Contrast": "최소 대비",
    "Modal Max Width": "모달 최대 너비",
    "Mode": "모드",
    "Multi Cursor Modifier": "다중 커서 보조 키",
    "New Thread Location": "새 스레드 위치",
    "Notifications": "알림",
    "Notify When Agent Waiting": "에이전트 대기 시 알림",
    "Object Fit": "Object Fit",
    "On Create": "생성 시",
    "On Drop": "드롭 시",
    "On Last Window Closed": "마지막 창 닫을 때",
    "On Paste": "붙여넣기 시",
    "Option As Meta": "Option을 Meta로",
    "Options": "옵션",
    "Outline Panel Button": "아웃라인 패널 버튼",
    "Outline Panel Default Width": "아웃라인 패널 기본 너비",
    "Outline Panel Dock": "아웃라인 패널 위치",
    "Padding": "여백",
    "Panels": "패널",
    "Parser": "파서",
    "Path Style": "경로 스타일",
    "Pinned Tabs Layout": "고정 탭 레이아웃",
    "Play Sound When Agent Done": "에이전트 완료 시 소리 재생",
    "Plugins": "플러그인",
    "Prefer LSP": "LSP 우선",
    "Preferred Line Length": "선호 라인 길이",
    "Preview Tabs Enabled": "미리보기 탭 사용",
    "Program": "프로그램",
    "Project Name": "프로젝트 이름",
    "Project Panel Button": "프로젝트 패널 버튼",
    "Project Panel Default Width": "프로젝트 패널 기본 너비",
    "Project Panel Dock": "프로젝트 패널 위치",
    "Project Search Button": "프로젝트 검색 버튼",
    "Quick Actions": "빠른 액션",
    "Regex": "정규식",
    "Relative Line Numbers": "상대 줄 번호",
    "Remove Trailing Whitespace On Save": "저장 시 후행 공백 제거",
    "Restore File State": "파일 상태 복원",
    "Restore On Startup": "시작 시 복원",
    "Restore Unsaved Buffers": "저장되지 않은 버퍼 복원",
    "Rounded Selection": "둥근 선택",
    "Scroll Bar": "스크롤바",
    "Scroll Beyond Last Line": "마지막 줄 이후 스크롤",
    "Scroll Debounce Ms": "스크롤 디바운스 (ms)",
    "Scroll Multiplier": "스크롤 배율",
    "Scroll Sensitivity": "스크롤 감도",
    "Search & Files": "검색 및 파일",
    "Search Results": "검색 결과",
    "Search Wrap": "검색 순환",
    "Seed Search Query From Cursor": "커서 위치에서 검색어 시드",
    "Selected Symbol": "선택한 심볼",
    "Selected Text": "선택한 텍스트",
    "Selection Highlight": "선택 강조",
    "Selections Menu": "선택 메뉴",
    "Semantic Tokens": "시맨틱 토큰",
    "Shell": "셸",
    "Show Author Name": "작성자 이름 표시",
    "Show Avatar": "아바타 표시",
    "Show Background": "배경 표시",
    "Show Close Button": "닫기 버튼 표시",
    "Show Commit Summary": "커밋 요약 표시",
    "Show Completion Documentation": "자동 완성 문서 표시",
    "Show Completions On Input": "입력 시 자동 완성 표시",
    "Show Count Badge": "개수 배지 표시",
    "Show Diagnostics": "진단 표시",
    "Show Edit Predictions": "편집 예측 표시",
    "Show File Icons In Tabs": "탭에 파일 아이콘 표시",
    "Show Folds": "접기 표시",
    "Show Git Status In Tabs": "탭에 Git 상태 표시",
    "Show Indent Guides": "들여쓰기 가이드 표시",
    "Show Line Numbers": "줄 번호 표시",
    "Show Navigation History Buttons": "탐색 기록 버튼 표시",
    "Show Onboarding Banner": "온보딩 배너 표시",
    "Show Other Hints": "기타 힌트 표시",
    "Show Parameter Hints": "매개변수 힌트 표시",
    "Show Runnables": "실행 가능 항목 표시",
    "Show Scrollbar": "스크롤바 표시",
    "Show Signature Help After Edits": "편집 후 시그니처 도움말 표시",
    "Show Tab Bar Buttons": "탭 표시줄 버튼 표시",
    "Show Tab Bar": "탭 표시줄 표시",
    "Show Turn Stats": "턴 통계 표시",
    "Show Type Hints": "타입 힌트 표시",
    "Show Value Hints": "값 힌트 표시",
    "Show Which-key Menu": "Which-key 메뉴 표시",
    "Show Whitespaces": "공백 표시",
    "Show Wrap Guides": "줄바꿈 가이드 표시",
    "Show": "표시",
    "Single File Review": "단일 파일 검토",
    "Skip Focus For Active In Search": "검색에서 활성 항목 포커스 건너뛰기",
    "Snippet Sort Order": "스니펫 정렬 순서",
    "Soft Wrap": "소프트 랩",
    "Sort By Path": "경로별 정렬",
    "Sort Mode": "정렬 모드",
    "Space Whitespace Indicator": "공백 표시기",
    "Starts Open": "시작 시 열기",
    "Sticky Scroll": "고정 스크롤",
    "System Monitoring": "시스템 모니터링",
    "Tab Close Position": "탭 닫기 위치",
    "Tab Show Diagnostics": "탭 진단 표시",
    "Tab Size": "탭 크기",
    "Tab Whitespace Indicator": "탭 공백 표시기",
    "Terminal Button": "터미널 버튼",
    "Terminal Dock": "터미널 위치",
    "Terminal": "터미널",
    "Text Rendering Mode": "텍스트 렌더링 모드",
    "Theme Mode": "테마 모드",
    "Theme Name": "테마 이름",
    "Thumb Border": "썸 테두리",
    "Thumb": "썸",
    "Title Override": "제목 재정의",
    "Toggle On Modifiers Press": "보조 키 누름으로 토글",
    "Toggle Relative Line Numbers": "상대 줄 번호 토글",
    "Tool Permissions": "도구 권한",
    "Tree View": "트리 보기",
    "UI Font Size": "UI 글꼴 크기",
    "Unnecessary Code Fade": "불필요한 코드 흐림",
    "Update Debounce": "업데이트 디바운스",
    "Use Auto Surround": "자동 둘러싸기 사용",
    "Use Autoclose": "자동 닫기 사용",
    "Use Modifier To Send": "보조 키로 전송",
    "Use On Type Format": "입력 시 포매팅 사용",
    "Use Smartcase Find": "Smartcase 찾기 사용",
    "Use Smartcase Search": "Smartcase 검색 사용",
    "Use System Clipboard": "시스템 클립보드 사용",
    "Use System Path Prompts": "시스템 경로 프롬프트 사용",
    "Use System Prompts": "시스템 프롬프트 사용",
    "Use System Window Tabs": "시스템 창 탭 사용",
    "Variables": "변수",
    "Version Control": "버전 관리",
    "Vertical Scroll Margin": "세로 스크롤 여백",
    "Vertical Scrollbar": "세로 스크롤바",
    "Vertical Split Direction": "세로 분할 방향",
    "Vim Mode": "Vim 모드",
    "Vim/Emacs Modeline Support": "Vim/Emacs 모드라인 지원",
    "Visibility": "표시 여부",
    "Wallpaper": "배경 화면",
    "When Closing With No Tabs": "탭이 없을 때 닫기",
    "Whole Word": "전체 단어",
    "Window & Layout": "창 및 레이아웃",
    "Window Decorations": "창 장식",
    "Word Diff Enabled": "단어 Diff 사용",
    "Words Min Length": "단어 최소 길이",
    "Words": "단어",
    "Working Directory": "작업 디렉토리",
    "Wrap Guides": "줄바꿈 가이드",
    "Zoomed Padding": "확대 시 여백",
}


def to_snake_key(s: str) -> str:
    """영문 title을 snake_case key로 변환"""
    s = s.lower()
    # &, /, - 등을 _로 치환
    s = re.sub(r"[&/\-]", "_", s)
    # 알파벳/숫자/_ 외 모두 제거
    s = re.sub(r"[^a-z0-9_]", "_", s)
    # 연속 _ 압축
    s = re.sub(r"_+", "_", s)
    # 앞뒤 _ 제거
    return s.strip("_")


def main():
    items = sorted(MAPPING.items())

    # 출력 모드 분기
    mode = sys.argv[1] if len(sys.argv) > 1 else "verify"

    if mode == "verify":
        # 키 충돌 검증
        keys = {}
        for en, ko in items:
            k = to_snake_key(en)
            if k in keys:
                print(f"COLLISION: {keys[k]} <-> {en} -> {k}")
            keys[k] = en
        print(f"OK: {len(items)} unique titles, {len(keys)} unique keys")

    elif mode == "en":
        # en.json 청크 출력
        import json
        for en, ko in items:
            k = to_snake_key(en)
            print(f'  "settings_page.item.{k}": {json.dumps(en, ensure_ascii=False)},')

    elif mode == "ko":
        # ko.json 청크 출력 (한글)
        for en, ko in items:
            k = to_snake_key(en)
            # JSON 안전 인코딩 위해 json 모듈 사용
            import json
            print(f'  "settings_page.item.{k}": {json.dumps(ko, ensure_ascii=False)},')

    elif mode == "edits":
        # page_data.rs 교체용 sed 스크립트 출력
        for en, ko in items:
            k = to_snake_key(en)
            old = f'title: "{en}"'
            new = f'title: "settings_page.item.{k}"'
            # bash sed 호환 (특수문자 escape 필요)
            print(f"OLD: {old}")
            print(f"NEW: {new}")
            print()

    elif mode == "json":
        # JSON 형태로 매핑 출력 (Edit 도구가 활용)
        import json
        out = {}
        for en, ko in items:
            k = to_snake_key(en)
            out[en] = {"key": f"settings_page.item.{k}", "ko": ko}
        print(json.dumps(out, ensure_ascii=False, indent=2))

    elif mode == "apply":
        # ko.json/en.json/page_data.rs 일괄 수정
        import json
        from pathlib import Path

        root = Path(r"D:\Personal Project\Windows\Dokkaebi")
        en_path = root / "assets/locales/en.json"
        ko_path = root / "assets/locales/ko.json"
        rs_path = root / "crates/settings_ui/src/page_data.rs"

        # 청크 생성
        en_lines = []
        ko_lines = []
        rs_replacements = []
        for en, ko in items:
            k = to_snake_key(en)
            en_lines.append(f'  "settings_page.item.{k}": {json.dumps(en, ensure_ascii=False)},')
            ko_lines.append(f'  "settings_page.item.{k}": {json.dumps(ko, ensure_ascii=False)},')
            rs_replacements.append((f'title: "{en}"', f'title: "settings_page.item.{k}"'))

        en_chunk = "\n".join(en_lines) + "\n"
        ko_chunk = "\n".join(ko_lines) + "\n"

        # en.json 수정 - settings_page.section.wrapping 다음 줄에 청크 삽입
        en_text = en_path.read_text(encoding="utf-8")
        marker_en = '  "settings_page.section.wrapping": "Wrapping",\n'
        if marker_en not in en_text:
            print(f"ERROR: marker not found in en.json: {marker_en!r}")
            sys.exit(1)
        en_text = en_text.replace(marker_en, marker_en + en_chunk, 1)
        en_path.write_text(en_text, encoding="utf-8")
        print(f"OK: en.json updated (+{len(en_lines)} keys)")

        # ko.json 수정 - settings_page.section.wrapping 다음 줄에 청크 삽입
        ko_text = ko_path.read_text(encoding="utf-8")
        marker_ko = '  "settings_page.section.wrapping": "줄바꿈",\n'
        if marker_ko not in ko_text:
            print(f"ERROR: marker not found in ko.json: {marker_ko!r}")
            sys.exit(1)
        ko_text = ko_text.replace(marker_ko, marker_ko + ko_chunk, 1)
        ko_path.write_text(ko_text, encoding="utf-8")
        print(f"OK: ko.json updated (+{len(ko_lines)} keys)")

        # page_data.rs 일괄 교체
        rs_text = rs_path.read_text(encoding="utf-8")
        replaced = 0
        not_found = []
        for old, new in rs_replacements:
            if old not in rs_text:
                not_found.append(old)
                continue
            count = rs_text.count(old)
            rs_text = rs_text.replace(old, new)
            replaced += count
        rs_path.write_text(rs_text, encoding="utf-8")
        print(f"OK: page_data.rs updated ({replaced} title occurrences replaced)")
        if not_found:
            print(f"WARN: {len(not_found)} titles not found in page_data.rs:")
            for n in not_found:
                print(f"  - {n}")


if __name__ == "__main__":
    main()
