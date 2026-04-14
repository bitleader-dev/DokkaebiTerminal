"""
SettingItem description을 D2 컨벤션 i18n 키로 일괄 전환.
- 컨벤션: settings_page.desc.<section>.<item>
- 톤: T1 격식체 (~합니다 종결)
- Zed 언급: Z3 일반화 (영문/한글 모두 "the app" 또는 빼기)
"""
import json
import re
import sys
from pathlib import Path

# (section, item, en, ko)
# en은 page_data.rs의 raw 영문 (원본 그대로, Zed 포함). page_data.rs 매칭에 사용.
# en_value는 i18n 키 값으로 들어갈 영문 (Z3 적용된 깨끗한 영문). 미지정 시 en 그대로 사용.
ENTRIES = [
    # === 카테고리 1: keymap_page (3) ===
    ('base_keymap', 'base_keymap',
     "The name of a base set of key bindings to use.",
     "사용할 기본 키 바인딩 세트의 이름입니다."),
    ('modal_editing', 'vim_mode',
     "Enable Vim mode and key bindings.",
     "Vim 모드 및 키 바인딩을 사용합니다."),
    ('modal_editing', 'helix_mode',
     "Enable Helix mode and key bindings.",
     "Helix 모드 및 키 바인딩을 사용합니다."),

    # === 카테고리 2: general_page (9) ===
    ('general_settings', 'project_name',
     "The displayed name of this project. If left empty, the root directory name will be displayed.",
     "프로젝트의 표시 이름입니다. 비워두면 루트 디렉토리 이름이 표시됩니다."),
    ('general_settings', 'when_closing_with_no_tabs',
     "What to do when using the 'close active item' action with no tabs.",
     "탭이 없는 상태에서 '활성 항목 닫기' 액션을 사용할 때의 동작입니다."),
    ('general_settings', 'on_last_window_closed',
     "What to do when the last window is closed.",
     "마지막 창이 닫힐 때의 동작입니다."),
    ('general_settings', 'use_system_path_prompts',
     "Use native OS dialogs for 'Open' and 'Save As'.",
     "'열기' 및 '다른 이름으로 저장'에 OS 기본 대화상자를 사용합니다."),
    ('general_settings', 'use_system_prompts',
     "Use native OS dialogs for confirmations.",
     "확인 동작에 OS 기본 대화상자를 사용합니다."),
    ('general_settings', 'system_monitoring',
     "Show CPU, memory, and GPU usage in the status bar.",
     "상태 표시줄에 CPU, 메모리, GPU 사용량을 표시합니다."),
    ('workspace_restoration', 'restore_unsaved_buffers',
     "Whether or not to restore unsaved buffers on restart.",
     "재시작 시 저장되지 않은 버퍼를 복원할지 여부입니다."),
    ('workspace_restoration', 'restore_on_startup',
     "What to restore from the previous session when opening Zed.",
     "앱 시작 시 이전 세션에서 복원할 항목입니다."),
    ('auto_update', 'auto_update',
     "Checks for the latest updates when the app starts.",
     "앱 시작 시 최신 업데이트를 확인합니다."),

    # === 카테고리 3: languages_and_tools_page (10) ===
    ('file_types', 'file_type_associations',
     "A mapping from languages to files and file extensions that should be treated as that language.",
     "특정 언어로 처리할 파일 및 파일 확장자를 언어와 매핑합니다."),
    ('diagnostics', 'max_severity',
     "Which level to use to filter out diagnostics displayed in the editor.",
     "편집기에 표시되는 진단을 필터링할 수준입니다."),
    ('diagnostics', 'include_warnings',
     "Whether to show warnings or not by default.",
     "기본적으로 경고를 표시할지 여부입니다."),
    ('inline_diagnostics', 'enabled',
     "Whether to show diagnostics inline or not.",
     "진단을 인라인으로 표시할지 여부입니다."),
    ('inline_diagnostics', 'update_debounce',
     "The delay in milliseconds to show inline diagnostics after the last diagnostic update.",
     "마지막 진단 업데이트 이후 인라인 진단을 표시하기까지의 지연 시간(밀리초)입니다."),
    ('inline_diagnostics', 'padding',
     "The amount of padding between the end of the source line and the start of the inline diagnostic.",
     "소스 줄 끝과 인라인 진단 시작 사이의 여백 크기입니다."),
    ('inline_diagnostics', 'minimum_column',
     "The minimum column at which to display inline diagnostics.",
     "인라인 진단을 표시할 최소 컬럼입니다."),
    ('lsp_pull_diagnostics', 'enabled',
     "Whether to pull for language server-powered diagnostics or not.",
     "언어 서버 기반 진단을 가져올지 여부입니다."),
    ('lsp_pull_diagnostics', 'debounce',
     "Minimum time to wait before pulling diagnostics from the language server(s).",
     "언어 서버에서 진단을 가져오기 전 최소 대기 시간입니다."),
    ('lsp_highlights', 'debounce',
     "The debounce delay before querying highlights from the language.",
     "언어에서 하이라이트를 조회하기 전 디바운스 지연 시간입니다."),

    # === 카테고리 4: version_control_page (14) ===
    ('git_integration', 'disable_git_integration',
     "Disable all Git integration features in Zed.",
     "모든 Git 통합 기능을 비활성화합니다."),
    ('git_integration', 'enable_git_status',
     "Show Git status information in the editor.",
     "편집기에 Git 상태 정보를 표시합니다."),
    ('git_integration', 'enable_git_diff',
     "Show Git diff information in the editor.",
     "편집기에 Git diff 정보를 표시합니다."),
    ('git_gutter', 'visibility',
     "Control whether Git status is shown in the editor's gutter.",
     "편집기 거터에 Git 상태를 표시할지 여부를 제어합니다."),
    ('git_gutter', 'debounce',
     "Debounce threshold in milliseconds after which changes are reflected in the Git gutter.",
     "변경사항이 Git 거터에 반영되기까지의 디바운스 임계값(밀리초)입니다."),
    ('inline_git_blame', 'enabled',
     "Whether or not to show Git blame data inline in the currently focused line.",
     "현재 포커스된 줄에 Git blame 데이터를 인라인으로 표시할지 여부입니다."),
    ('inline_git_blame', 'delay',
     "The delay after which the inline blame information is shown.",
     "인라인 blame 정보가 표시되기까지의 지연 시간입니다."),
    ('inline_git_blame', 'padding',
     "Padding between the end of the source line and the start of the inline blame in columns.",
     "소스 줄 끝과 인라인 blame 시작 사이의 여백(컬럼 단위)입니다."),
    ('inline_git_blame', 'minimum_column',
     "The minimum column number at which to show the inline blame information.",
     "인라인 blame 정보를 표시할 최소 컬럼 번호입니다."),
    ('inline_git_blame', 'show_commit_summary',
     "Show commit summary as part of the inline blame.",
     "인라인 blame에 커밋 요약을 표시합니다."),
    ('git_blame_view', 'show_avatar',
     "Show the avatar of the author of the commit.",
     "커밋 작성자의 아바타를 표시합니다."),
    ('branch_picker', 'show_author_name',
     "Show author name as part of the commit information in branch picker.",
     "브랜치 선택기의 커밋 정보에 작성자 이름을 표시합니다."),
    ('git_hunks', 'hunk_style',
     "How Git hunks are displayed visually in the editor.",
     "편집기에서 Git hunk를 시각적으로 표시하는 방식입니다."),
    ('git_hunks', 'path_style',
     "Should the name or path be displayed first in the git view.",
     "Git 보기에서 이름과 경로 중 어느 것을 먼저 표시할지 결정합니다."),

    # === 카테고리 5: ai_page (15) ===
    ('general', 'disable_ai',
     "Whether to disable all AI features in Zed.",
     "모든 AI 기능을 비활성화할지 여부입니다."),
    ('agent_configuration', 'new_thread_location',
     "Whether to start a new thread in the current local project or in a new Git worktree.",
     "현재 로컬 프로젝트 또는 새 Git worktree 중 어디에 새 스레드를 시작할지 결정합니다."),
    ('agent_configuration', 'single_file_review',
     "When enabled, agent edits will also be displayed in single-file buffers for review.",
     "사용 시 에이전트 편집이 검토를 위해 단일 파일 버퍼에도 표시됩니다."),
    ('agent_configuration', 'enable_feedback',
     "Show voting thumbs up/down icon buttons for feedback on agent edits.",
     "에이전트 편집에 대한 피드백 추천/비추천 버튼을 표시합니다."),
    ('agent_configuration', 'notify_when_agent_waiting',
     "Where to show notifications when the agent has completed its response or needs confirmation before running a tool action.",
     "에이전트가 응답을 완료하거나 도구 액션 실행 전 확인이 필요할 때 알림을 표시할 위치입니다."),
    ('agent_configuration', 'play_sound_when_agent_done',
     "Whether to play a sound when the agent has either completed its response, or needs user input.",
     "에이전트가 응답을 완료하거나 사용자 입력이 필요할 때 소리를 재생할지 여부입니다."),
    ('agent_configuration', 'expand_edit_card',
     "Whether to have edit cards in the agent panel expanded, showing a Preview of the diff.",
     "에이전트 패널의 편집 카드를 확장하여 diff 미리보기를 표시할지 여부입니다."),
    ('agent_configuration', 'expand_terminal_card',
     "Whether to have terminal cards in the agent panel expanded, showing the whole command output.",
     "에이전트 패널의 터미널 카드를 확장하여 전체 명령 출력을 표시할지 여부입니다."),
    ('agent_configuration', 'cancel_generation_on_terminal_stop',
     "Whether clicking the stop button on a running terminal tool should also cancel the agent's generation. Note that this only applies to the stop button, not to ctrl+c inside the terminal.",
     "실행 중인 터미널 도구의 정지 버튼을 클릭할 때 에이전트 생성도 함께 취소할지 여부입니다. 이 옵션은 정지 버튼에만 적용되며 터미널 내부의 ctrl+c에는 적용되지 않습니다."),
    ('agent_configuration', 'use_modifier_to_send',
     "Whether to always use cmd-enter (or ctrl-enter on Linux or Windows) to send messages.",
     "메시지 전송에 항상 cmd-enter(Linux/Windows에서는 ctrl-enter)를 사용할지 여부입니다."),
    ('agent_configuration', 'message_editor_min_lines',
     "Minimum number of lines to display in the agent message editor.",
     "에이전트 메시지 편집기에 표시할 최소 줄 수입니다."),
    ('agent_configuration', 'show_turn_stats',
     "Whether to show turn statistics like elapsed time during generation and final turn duration.",
     "생성 중 경과 시간 및 최종 턴 소요 시간 등 턴 통계를 표시할지 여부입니다."),
    ('context_servers', 'context_server_timeout',
     "Default timeout in seconds for context server tool calls. Can be overridden per-server in context_servers configuration.",
     "컨텍스트 서버 도구 호출의 기본 타임아웃(초)입니다. context_servers 설정에서 서버별로 재정의할 수 있습니다."),
    ('edit_prediction_display_sub', 'display_mode',
     "When to show edit predictions previews in buffer. The eager mode displays them inline, while the subtle mode displays them only when holding a modifier key.",
     "버퍼에 편집 예측 미리보기를 표시할 시점입니다. eager 모드는 인라인으로 표시하고, subtle 모드는 보조 키를 누를 때만 표시합니다."),
    ('edit_prediction_display_sub', 'display_in_text_threads',
     "Whether edit predictions are enabled when editing text threads in the agent panel.",
     "에이전트 패널에서 텍스트 스레드 편집 시 편집 예측을 사용할지 여부입니다."),

    # === 카테고리 6: search_and_files_page (16) ===
    ('search', 'whole_word',
     "Search for whole words by default.",
     "기본적으로 전체 단어로 검색합니다."),
    ('search', 'case_sensitive',
     "Search case-sensitively by default.",
     "기본적으로 대소문자를 구분하여 검색합니다."),
    ('search', 'use_smartcase_search',
     "Whether to automatically enable case-sensitive search based on the search query.",
     "검색어에 따라 대소문자 구분 검색을 자동으로 활성화할지 여부입니다."),
    ('search', 'include_ignored',
     "Include ignored files in search results by default.",
     "기본적으로 검색 결과에 무시된 파일을 포함합니다."),
    ('search', 'regex',
     "Use regex search by default.",
     "기본적으로 정규식 검색을 사용합니다."),
    ('search', 'search_wrap',
     "Whether the editor search results will loop.",
     "편집기 검색 결과가 순환할지 여부입니다."),
    ('search', 'center_on_match',
     "Whether to center the current match in the editor",
     "현재 일치 항목을 편집기 중앙에 정렬할지 여부입니다."),
    ('search', 'seed_search_query_from_cursor',
     "When to populate a new search's query based on the text under the cursor.",
     "새 검색의 검색어를 커서 위치의 텍스트로 채울 시점입니다."),
    ('file_finder', 'include_ignored_in_search',
     "Use gitignored files when searching.",
     "검색 시 gitignore 파일을 포함합니다."),
    ('file_finder', 'file_icons',
     "Show file icons in the file finder.",
     "파일 찾기에 파일 아이콘을 표시합니다."),
    ('file_finder', 'modal_max_width',
     "Determines how much space the file finder can take up in relation to the available window width.",
     "파일 찾기가 사용 가능한 창 너비에서 차지할 공간을 결정합니다."),
    ('file_finder', 'skip_focus_for_active_in_search',
     "Whether the file finder should skip focus for the active file in search results.",
     "파일 찾기에서 검색 결과의 활성 파일에 대한 포커스를 건너뛸지 여부입니다."),
    ('file_scan', 'file_scan_exclusions',
     'Files or globs of files that will be excluded by Zed entirely. They will be skipped during file scans, file searches, and not be displayed in the project file tree. Takes precedence over \\"File Scan Inclusions\\"',
     "전체적으로 제외할 파일 또는 파일 globs입니다. 파일 스캔 및 검색 시 건너뛰며 프로젝트 파일 트리에도 표시되지 않습니다. \"파일 스캔 포함\"보다 우선합니다."),
    ('file_scan', 'file_scan_inclusions',
     'Files or globs of files that will be included by Zed, even when ignored by git. This is useful for files that are not tracked by git, but are still important to your project. Note that globs that are overly broad can slow down Zed\'s file scanning. \\"File Scan Exclusions\\" takes precedence over these inclusions',
     "git이 무시한 경우에도 포함할 파일 또는 파일 globs입니다. git에 추적되지 않지만 프로젝트에 중요한 파일에 유용합니다. 너무 광범위한 glob은 파일 스캔 속도를 늦출 수 있습니다. \"파일 스캔 제외\"가 이 포함보다 우선합니다."),
    ('file_scan', 'restore_file_state',
     "Restore previous file state when reopening.",
     "재오픈 시 이전 파일 상태를 복원합니다."),
    ('file_scan', 'close_on_file_delete',
     "Automatically close files that have been deleted.",
     "삭제된 파일을 자동으로 닫습니다."),

    # === 카테고리 7: terminal_page (28) ===
    # toolbar/scrollbar는 editor_page와 section 이름 충돌 회피 위해 terminal_ prefix
    ('environment', 'shell', "What shell to use when opening a terminal.", "터미널을 열 때 사용할 셸입니다."),
    ('environment', 'program', "The shell program to use.", "사용할 셸 프로그램입니다."),
    ('environment', 'program', "The shell program to run.", "사용할 셸 프로그램입니다."),  # 같은 의미, 통합
    ('environment', 'arguments', "The arguments to pass to the shell program.", "셸 프로그램에 전달할 인자입니다."),
    ('environment', 'title_override', "An optional string to override the title of the terminal tab.", "터미널 탭 제목을 재정의할 선택적 문자열입니다."),
    ('environment', 'working_directory', "What working directory to use when launching the terminal.", "터미널을 시작할 때 사용할 작업 디렉토리입니다."),
    ('environment', 'directory', "The directory path to use (will be shell expanded).", "사용할 디렉토리 경로입니다(셸 확장됨)."),
    ('environment', 'environment_variables', "Key-value pairs to add to the terminal's environment.", "터미널 환경에 추가할 키-값 쌍입니다."),
    ('environment', 'detect_virtual_environment', "Activates the Python virtual environment, if one is found, in the terminal's working directory.", "터미널 작업 디렉토리에서 Python 가상 환경을 발견하면 활성화합니다."),
    ('font', 'font_size', "Font size for terminal text. If not set, defaults to buffer font size.", "터미널 텍스트의 글꼴 크기입니다. 설정하지 않으면 버퍼 글꼴 크기를 사용합니다."),
    ('font', 'font_family', "Font family for terminal text. If not set, defaults to buffer font family.", "터미널 텍스트의 글꼴입니다. 설정하지 않으면 버퍼 글꼴을 사용합니다."),
    ('font', 'font_fallbacks', "Font fallbacks for terminal text. If not set, defaults to buffer font fallbacks.", "터미널 텍스트의 글꼴 대체입니다. 설정하지 않으면 버퍼 글꼴 대체를 사용합니다."),
    ('font', 'font_weight', "Font weight for terminal text in CSS weight units (100-900).", "CSS 굵기 단위(100-900)로 지정한 터미널 텍스트의 글꼴 굵기입니다."),
    ('font', 'font_features', "Font features for terminal text.", "터미널 텍스트의 글꼴 기능입니다."),
    ('display_settings', 'line_height', "Line height for terminal text.", "터미널 텍스트의 라인 높이입니다."),
    ('display_settings', 'cursor_shape', "Default cursor shape for the terminal (bar, block, underline, or hollow).", "터미널 기본 커서 모양입니다(bar, block, underline, hollow)."),
    ('display_settings', 'cursor_blinking', "Sets the cursor blinking behavior in the terminal.", "터미널의 커서 깜빡임 동작을 설정합니다."),
    ('display_settings', 'alternate_scroll', "Whether alternate scroll mode is active by default (converts mouse scroll to arrow keys in apps like Vim).", "기본적으로 대체 스크롤 모드를 활성화할지 여부입니다(Vim 같은 앱에서 마우스 스크롤을 화살표 키로 변환)."),
    ('display_settings', 'minimum_contrast', "The minimum APCA perceptual contrast between foreground and background colors (0-106).", "전경색과 배경색 간 최소 APCA 인지 대비입니다(0-106)."),
    ('behavior_settings', 'option_as_meta', "Whether the option key behaves as the meta key.", "Option 키를 Meta 키처럼 동작시킬지 여부입니다."),
    ('behavior_settings', 'copy_on_select', "Whether selecting text in the terminal automatically copies to the system clipboard.", "터미널에서 텍스트를 선택하면 시스템 클립보드로 자동 복사할지 여부입니다."),
    ('behavior_settings', 'keep_selection_on_copy', "Whether to keep the text selection after copying it to the clipboard.", "클립보드에 복사한 후 텍스트 선택을 유지할지 여부입니다."),
    ('layout_settings', 'default_width', "Default width when the terminal is docked to the left or right (in pixels).", "터미널을 좌측 또는 우측에 도킹할 때의 기본 너비(픽셀)입니다."),
    ('layout_settings', 'default_height', "Default height when the terminal is docked to the bottom (in pixels).", "터미널을 하단에 도킹할 때의 기본 높이(픽셀)입니다."),
    ('advanced_settings', 'max_scroll_history_lines', "Maximum number of lines to keep in scrollback history (max: 100,000; 0 disables scrolling).", "스크롤백 기록에 유지할 최대 줄 수입니다(최대 100,000, 0이면 스크롤 비활성화)."),
    ('advanced_settings', 'scroll_multiplier', "The multiplier for scrolling in the terminal with the mouse wheel", "마우스 휠로 터미널에서 스크롤할 때의 배율입니다."),
    ('terminal_toolbar', 'breadcrumbs', "Display the terminal title in breadcrumbs inside the terminal pane.", "터미널 페인 내 경로 표시줄에 터미널 제목을 표시합니다."),
    ('terminal_scrollbar', 'show_scrollbar', "When to show the scrollbar in the terminal.", "터미널에서 스크롤바를 표시할 시점입니다."),

    # === 카테고리 8: window_and_layout_page (35) ===
    ('status_bar', 'project_panel_button', "Show the project panel button in the status bar.", "상태 표시줄에 프로젝트 패널 버튼을 표시합니다."),
    ('status_bar', 'active_language_button', "Show the active language button in the status bar.", "상태 표시줄에 현재 언어 버튼을 표시합니다."),
    ('status_bar', 'active_encoding_button', "Control when to show the active encoding in the status bar.", "상태 표시줄에 현재 인코딩을 표시할 시점을 제어합니다."),
    ('status_bar', 'cursor_position_button', "Show the cursor position button in the status bar.", "상태 표시줄에 커서 위치 버튼을 표시합니다."),
    ('status_bar', 'terminal_button', "Show the terminal button in the status bar.", "상태 표시줄에 터미널 버튼을 표시합니다."),
    ('status_bar', 'diagnostics_button', "Show the project diagnostics button in the status bar.", "상태 표시줄에 프로젝트 진단 버튼을 표시합니다."),
    ('status_bar', 'project_search_button', "Show the project search button in the status bar.", "상태 표시줄에 프로젝트 검색 버튼을 표시합니다."),
    ('status_bar', 'active_file_name', "Show the name of the active file in the status bar.", "상태 표시줄에 현재 파일 이름을 표시합니다."),
    ('title_bar', 'show_onboarding_banner', "Show banners announcing new features in the titlebar.", "제목 표시줄에 새 기능 안내 배너를 표시합니다."),
    ('tab_bar', 'show_tab_bar', "Show the tab bar in the editor.", "편집기에 탭 표시줄을 표시합니다."),
    ('tab_bar', 'show_git_status_in_tabs', "Show the Git file status on a tab item.", "탭 항목에 Git 파일 상태를 표시합니다."),
    ('tab_bar', 'show_file_icons_in_tabs', "Show the file icon for a tab.", "탭에 파일 아이콘을 표시합니다."),
    ('tab_bar', 'tab_close_position', "Position of the close button in a tab.", "탭의 닫기 버튼 위치입니다."),
    ('tab_bar', 'maximum_tabs', "Maximum open tabs in a pane. Will not close an unsaved tab.", "페인의 최대 열린 탭 수입니다. 저장되지 않은 탭은 닫지 않습니다."),
    ('tab_bar', 'show_navigation_history_buttons', "Show the navigation history buttons in the tab bar.", "탭 표시줄에 탐색 기록 버튼을 표시합니다."),
    ('tab_bar', 'show_tab_bar_buttons', "Show the tab bar buttons (New, Split Pane, Zoom).", "탭 표시줄 버튼(새로 만들기, 페인 분할, 확대)을 표시합니다."),
    ('tab_bar', 'pinned_tabs_layout', "Show pinned tabs in a separate row above unpinned tabs.", "고정되지 않은 탭 위에 별도 줄로 고정 탭을 표시합니다."),
    ('tab_settings', 'activate_on_close', "What to do after closing the current tab.", "현재 탭을 닫은 후의 동작입니다."),
    ('tab_settings', 'tab_show_diagnostics', "Which files containing diagnostic errors/warnings to mark in the tabs.", "탭에 표시할 진단 오류/경고가 포함된 파일 종류입니다."),
    ('tab_settings', 'show_close_button', "Controls the appearance behavior of the tab's close button.", "탭 닫기 버튼의 표시 동작을 제어합니다."),
    ('preview_tabs', 'preview_tabs_enabled', "Show opened editors as preview tabs.", "열린 편집기를 미리보기 탭으로 표시합니다."),
    ('preview_tabs', 'enable_preview_from_project_panel', "Whether to open tabs in preview mode when opened from the project panel with a single click.", "프로젝트 패널에서 한 번 클릭으로 열 때 미리보기 모드로 탭을 열지 여부입니다."),
    ('preview_tabs', 'enable_preview_from_file_finder', "Whether to open tabs in preview mode when selected from the file finder.", "파일 찾기에서 선택 시 미리보기 모드로 탭을 열지 여부입니다."),
    ('preview_tabs', 'enable_preview_from_multibuffer', "Whether to open tabs in preview mode when opened from a multibuffer.", "멀티버퍼에서 열 때 미리보기 모드로 탭을 열지 여부입니다."),
    ('preview_tabs', 'enable_preview_multibuffer_from_code_navigation', "Whether to open tabs in preview mode when code navigation is used to open a multibuffer.", "코드 탐색으로 멀티버퍼를 열 때 미리보기 모드로 탭을 열지 여부입니다."),
    ('preview_tabs', 'enable_preview_file_from_code_navigation', "Whether to open tabs in preview mode when code navigation is used to open a single file.", "코드 탐색으로 단일 파일을 열 때 미리보기 모드로 탭을 열지 여부입니다."),
    ('preview_tabs', 'enable_keep_preview_on_code_navigation', "Whether to keep tabs in preview mode when code navigation is used to navigate away from them. If `enable_preview_file_from_code_navigation` or `enable_preview_multibuffer_from_code_navigation` is also true, the new tab may replace the existing one.", "코드 탐색으로 탭을 떠날 때 미리보기 모드를 유지할지 여부입니다. `enable_preview_file_from_code_navigation` 또는 `enable_preview_multibuffer_from_code_navigation`도 true이면 새 탭이 기존 탭을 대체할 수 있습니다."),
    ('layout', 'bottom_dock_layout', "Layout mode for the bottom dock.", "하단 도크의 레이아웃 모드입니다."),
    ('layout', 'centered_layout_left_padding', "Left padding for centered layout.", "중앙 레이아웃의 왼쪽 여백입니다."),
    ('layout', 'centered_layout_right_padding', "Right padding for centered layout.", "중앙 레이아웃의 오른쪽 여백입니다."),
    ('pane_modifiers', 'inactive_opacity', "Opacity of inactive panels (0.0 - 1.0).", "비활성 패널의 투명도입니다(0.0 - 1.0)."),
    ('pane_modifiers', 'border_size', "Size of the border surrounding the active pane.", "활성 페인을 둘러싼 테두리 크기입니다."),
    ('pane_modifiers', 'zoomed_padding', "Show padding for zoomed panes.", "확대된 페인의 여백을 표시합니다."),
    ('pane_split_direction', 'vertical_split_direction', "Direction to split vertically.", "세로로 분할할 방향입니다."),
    ('pane_split_direction', 'horizontal_split_direction', "Direction to split horizontally.", "가로로 분할할 방향입니다."),

    # === 카테고리 9: appearance_page (36) ===
    # 두 번째 (theme, mode)는 icon_theme/mode로 분리 (다른 의미)
    ('theme', 'theme_mode', "Choose a static, fixed theme or dynamically select themes based on appearance and light/dark modes.", "정적 고정 테마를 선택하거나 외관과 라이트/다크 모드에 따라 테마를 동적으로 선택합니다."),
    ('theme', 'theme_name', "The name of your selected theme.", "선택한 테마의 이름입니다."),
    ('theme', 'mode', "Choose whether to use the selected light or dark theme or to follow your OS appearance configuration.", "선택한 라이트 또는 다크 테마를 사용할지, OS 외관 설정을 따를지 선택합니다."),
    ('theme', 'light_theme', "The theme to use when mode is set to light, or when mode is set to system and it is in light mode.", "모드가 light이거나 system인 동시에 라이트 모드일 때 사용할 테마입니다."),
    ('theme', 'dark_theme', "The theme to use when mode is set to dark, or when mode is set to system and it is in dark mode.", "모드가 dark이거나 system인 동시에 다크 모드일 때 사용할 테마입니다."),
    ('theme', 'icon_theme', "The custom set of icons Zed will associate with files and directories.", "파일과 디렉토리에 연결할 사용자 아이콘 세트입니다."),
    ('theme', 'icon_theme_name', "The name of your selected icon theme.", "선택한 아이콘 테마의 이름입니다."),
    ('icon_theme', 'mode', "Choose whether to use the selected light or dark icon theme or to follow your OS appearance configuration.", "선택한 라이트 또는 다크 아이콘 테마를 사용할지, OS 외관 설정을 따를지 선택합니다."),
    ('theme', 'light_icon_theme', "The icon theme to use when mode is set to light, or when mode is set to system and it is in light mode.", "모드가 light이거나 system인 동시에 라이트 모드일 때 사용할 아이콘 테마입니다."),
    ('theme', 'dark_icon_theme', "The icon theme to use when mode is set to dark, or when mode is set to system and it is in dark mode.", "모드가 dark이거나 system인 동시에 다크 모드일 때 사용할 아이콘 테마입니다."),
    ('buffer_font', 'font_family', "Font family for editor text.", "편집기 텍스트의 글꼴입니다."),
    ('buffer_font', 'font_size', "Font size for editor text.", "편집기 텍스트의 글꼴 크기입니다."),
    ('buffer_font', 'font_weight', "Font weight for editor text (100-900).", "편집기 텍스트의 글꼴 굵기입니다(100-900)."),
    ('buffer_font', 'line_height', "Line height for editor text.", "편집기 텍스트의 라인 높이입니다."),
    ('buffer_font', 'custom_line_height', "Custom line height value (must be at least 1.0).", "사용자 지정 라인 높이 값입니다(최소 1.0)."),
    ('buffer_font', 'font_features', "The OpenType features to enable for rendering in text buffers.", "텍스트 버퍼 렌더링에 사용할 OpenType 기능입니다."),
    ('buffer_font', 'font_fallbacks', "The font fallbacks to use for rendering in text buffers.", "텍스트 버퍼 렌더링에 사용할 글꼴 대체입니다."),
    ('ui_font', 'font_family', "Font family for UI elements.", "UI 요소의 글꼴입니다."),
    ('ui_font', 'font_size', "Font size for UI elements.", "UI 요소의 글꼴 크기입니다."),
    ('ui_font', 'font_weight', "Font weight for UI elements (100-900).", "UI 요소의 글꼴 굵기입니다(100-900)."),
    ('ui_font', 'font_features', "The OpenType features to enable for rendering in UI elements.", "UI 요소 렌더링에 사용할 OpenType 기능입니다."),
    ('ui_font', 'font_fallbacks', "The font fallbacks to use for rendering in the UI.", "UI 렌더링에 사용할 글꼴 대체입니다."),
    ('agent_panel_font', 'ui_font_size', "Font size for agent response text in the agent panel. Falls back to the regular UI font size.", "에이전트 패널의 에이전트 응답 텍스트 글꼴 크기입니다. 미지정 시 기본 UI 글꼴 크기를 사용합니다."),
    ('agent_panel_font', 'buffer_font_size', "Font size for user messages text in the agent panel.", "에이전트 패널의 사용자 메시지 텍스트 글꼴 크기입니다."),
    ('text_rendering', 'text_rendering_mode', "The text rendering mode to use.", "사용할 텍스트 렌더링 모드입니다."),
    ('cursor', 'multi_cursor_modifier', "Modifier key for adding multiple cursors.", "다중 커서를 추가할 보조 키입니다."),
    ('cursor', 'cursor_blink', "Whether the cursor blinks in the editor.", "편집기 커서가 깜빡일지 여부입니다."),
    ('cursor', 'cursor_shape', "Cursor shape for the editor.", "편집기의 커서 모양입니다."),
    ('cursor', 'hide_mouse', "When to hide the mouse cursor.", "마우스 커서를 숨길 시점입니다."),
    ('highlighting', 'unnecessary_code_fade', "How much to fade out unused code (0.0 - 0.9).", "사용하지 않는 코드를 흐리게 표시할 정도입니다(0.0 - 0.9)."),
    ('highlighting', 'current_line_highlight', "How to highlight the current line.", "현재 라인을 강조하는 방식입니다."),
    ('highlighting', 'selection_highlight', "Highlight all occurrences of selected text.", "선택한 텍스트의 모든 출현을 강조합니다."),
    ('highlighting', 'rounded_selection', "Whether the text selection should have rounded corners.", "텍스트 선택의 모서리를 둥글게 표시할지 여부입니다."),
    ('highlighting', 'minimum_contrast_for_highlights', "The minimum APCA perceptual contrast to maintain when rendering text over highlight backgrounds.", "강조 배경 위에 텍스트를 렌더링할 때 유지할 최소 APCA 인지 대비입니다."),
    ('guides', 'show_wrap_guides', "Show wrap guides (vertical rulers).", "줄바꿈 가이드(수직 눈금자)를 표시합니다."),
    ('guides', 'wrap_guides', "Character counts at which to show wrap guides.", "줄바꿈 가이드를 표시할 글자 수입니다."),

    # === 카테고리 10: panels_page (56) ===
    ('project_panel', 'project_panel_dock', "Where to dock the project panel.", "프로젝트 패널을 도킹할 위치입니다."),
    ('project_panel', 'project_panel_default_width', "Default width of the project panel in pixels.", "프로젝트 패널의 기본 너비(픽셀)입니다."),
    ('project_panel', 'hide_gitignore', "Whether to hide the gitignore entries in the project panel.", "프로젝트 패널에서 gitignore 항목을 숨길지 여부입니다."),
    ('project_panel', 'entry_spacing', "Spacing between worktree entries in the project panel.", "프로젝트 패널의 worktree 항목 간 간격입니다."),
    ('project_panel', 'file_icons', "Show file icons in the project panel.", "프로젝트 패널에 파일 아이콘을 표시합니다."),
    ('project_panel', 'folder_icons', "Whether to show folder icons or chevrons for directories in the project panel.", "프로젝트 패널의 디렉토리에 폴더 아이콘 또는 화살표를 표시할지 여부입니다."),
    ('project_panel', 'git_status', "Show the Git status in the project panel.", "프로젝트 패널에 Git 상태를 표시합니다."),
    ('project_panel', 'indent_size', "Amount of indentation for nested items.", "중첩된 항목의 들여쓰기 크기입니다."),
    ('project_panel', 'auto_reveal_entries', "Whether to reveal entries in the project panel automatically when a corresponding project entry becomes active.", "관련 프로젝트 항목이 활성화될 때 프로젝트 패널에서 항목을 자동으로 노출할지 여부입니다."),
    ('project_panel', 'starts_open', "Whether the project panel should open on startup.", "시작 시 프로젝트 패널을 열지 여부입니다."),
    ('project_panel', 'auto_fold_directories', "Whether to fold directories automatically and show compact folders when a directory has only one subdirectory inside.", "디렉토리에 하위 디렉토리가 하나만 있을 때 자동으로 접어 컴팩트 폴더로 표시할지 여부입니다."),
    ('project_panel', 'bold_folder_labels', "Whether to show folder names with bold text in the project panel.", "프로젝트 패널의 폴더 이름을 굵게 표시할지 여부입니다."),
    ('project_panel', 'show_scrollbar', "Show the scrollbar in the project panel.", "프로젝트 패널에 스크롤바를 표시합니다."),
    ('project_panel', 'horizontal_scroll', "Whether to allow horizontal scrolling in the project panel. When disabled, the view is always locked to the leftmost position and long file names are clipped.", "프로젝트 패널의 가로 스크롤 허용 여부입니다. 비활성화 시 보기는 항상 가장 왼쪽 위치에 고정되며 긴 파일 이름은 잘립니다."),
    ('project_panel', 'show_diagnostics', "Which files containing diagnostic errors/warnings to mark in the project panel.", "프로젝트 패널에 표시할 진단 오류/경고가 포함된 파일 종류입니다."),
    ('project_panel', 'diagnostic_badges', "Show error and warning count badges next to file names in the project panel.", "프로젝트 패널의 파일 이름 옆에 오류 및 경고 개수 배지를 표시합니다."),
    ('project_panel', 'git_status_indicator', "Show a git status indicator next to file names in the project panel.", "프로젝트 패널의 파일 이름 옆에 git 상태 표시기를 표시합니다."),
    ('project_panel', 'sticky_scroll', "Whether to stick parent directories at top of the project panel.", "프로젝트 패널 상단에 상위 디렉토리를 고정할지 여부입니다."),
    ('project_panel', 'show_indent_guides', "Show indent guides in the project panel.", "프로젝트 패널에 들여쓰기 가이드를 표시합니다."),
    ('project_panel', 'drag_and_drop', "Whether to enable drag-and-drop operations in the project panel.", "프로젝트 패널의 드래그 앤 드롭 동작을 활성화할지 여부입니다."),
    ('project_panel', 'hide_root', "Whether to hide the root entry when only one folder is open in the window.", "창에 폴더가 하나만 열려있을 때 루트 항목을 숨길지 여부입니다."),
    ('project_panel', 'hide_hidden', "Whether to hide the hidden entries in the project panel.", "프로젝트 패널의 숨김 항목을 숨길지 여부입니다."),
    ('project_panel', 'hidden_files', 'Globs to match files that will be considered \\"hidden\\" and can be hidden from the project panel.', "\"숨김\"으로 처리되어 프로젝트 패널에서 숨길 수 있는 파일을 일치시키는 globs입니다."),
    ('auto_open_files', 'on_create', "Whether to automatically open newly created files in the editor.", "새로 생성된 파일을 편집기에서 자동으로 열지 여부입니다."),
    ('auto_open_files', 'on_paste', "Whether to automatically open files after pasting or duplicating them.", "파일을 붙여넣거나 복제한 후 자동으로 열지 여부입니다."),
    ('auto_open_files', 'on_drop', "Whether to automatically open files dropped from external sources.", "외부에서 드롭한 파일을 자동으로 열지 여부입니다."),
    ('auto_open_files', 'sort_mode', "Sort order for entries in the project panel.", "프로젝트 패널 항목의 정렬 순서입니다."),
    ('terminal_panel', 'terminal_dock', "Where to dock the terminal panel.", "터미널 패널을 도킹할 위치입니다."),
    ('terminal_panel', 'show_count_badge', "Show a badge on the terminal panel icon with the count of open terminals.", "터미널 패널 아이콘에 열린 터미널 개수 배지를 표시합니다."),
    ('outline_panel', 'outline_panel_button', "Show the outline panel button in the status bar.", "상태 표시줄에 아웃라인 패널 버튼을 표시합니다."),
    ('outline_panel', 'outline_panel_dock', "Where to dock the outline panel.", "아웃라인 패널을 도킹할 위치입니다."),
    ('outline_panel', 'outline_panel_default_width', "Default width of the outline panel in pixels.", "아웃라인 패널의 기본 너비(픽셀)입니다."),
    ('outline_panel', 'file_icons', "Show file icons in the outline panel.", "아웃라인 패널에 파일 아이콘을 표시합니다."),
    ('outline_panel', 'folder_icons', "Whether to show folder icons or chevrons for directories in the outline panel.", "아웃라인 패널의 디렉토리에 폴더 아이콘 또는 화살표를 표시할지 여부입니다."),
    ('outline_panel', 'git_status', "Show the Git status in the outline panel.", "아웃라인 패널에 Git 상태를 표시합니다."),
    ('outline_panel', 'indent_size', "Amount of indentation for nested items.", "중첩된 항목의 들여쓰기 크기입니다."),
    ('outline_panel', 'auto_reveal_entries', "Whether to reveal when a corresponding outline entry becomes active.", "관련 아웃라인 항목이 활성화될 때 노출할지 여부입니다."),
    ('outline_panel', 'auto_fold_directories', "Whether to fold directories automatically when a directory contains only one subdirectory.", "디렉토리에 하위 디렉토리가 하나만 있을 때 자동으로 접을지 여부입니다."),
    ('outline_panel', 'show_indent_guides', "When to show indent guides in the outline panel.", "아웃라인 패널에 들여쓰기 가이드를 표시할 시점입니다."),
    ('git_panel', 'git_panel_button', "Show the Git panel button in the status bar.", "상태 표시줄에 Git 패널 버튼을 표시합니다."),
    ('git_panel', 'git_panel_dock', "Where to dock the Git panel.", "Git 패널을 도킹할 위치입니다."),
    ('git_panel', 'git_panel_default_width', "Default width of the Git panel in pixels.", "Git 패널의 기본 너비(픽셀)입니다."),
    ('git_panel', 'git_panel_status_style', "How entry statuses are displayed.", "항목 상태를 표시하는 방식입니다."),
    ('git_panel', 'fallback_branch_name', "Default branch name will be when init.defaultbranch is not set in Git.", "Git에 init.defaultbranch가 설정되지 않았을 때의 기본 브랜치 이름입니다."),
    ('git_panel', 'sort_by_path', "Enable to sort entries in the panel by path, disable to sort by status.", "활성화하면 패널 항목을 경로별로 정렬하고, 비활성화하면 상태별로 정렬합니다."),
    ('git_panel', 'collapse_untracked_diff', "Whether to collapse untracked files in the diff panel.", "diff 패널에서 추적되지 않은 파일을 접을지 여부입니다."),
    ('git_panel', 'tree_view', "Enable to show entries in tree view list, disable to show in flat view list.", "활성화하면 항목을 트리 보기 목록으로, 비활성화하면 평면 보기 목록으로 표시합니다."),
    ('git_panel', 'file_icons', "Show file icons next to the Git status icon.", "Git 상태 아이콘 옆에 파일 아이콘을 표시합니다."),
    ('git_panel', 'folder_icons', "Whether to show folder icons or chevrons for directories in the git panel.", "git 패널의 디렉토리에 폴더 아이콘 또는 화살표를 표시할지 여부입니다."),
    ('git_panel', 'diff_stats', "Whether to show the addition/deletion change count next to each file in the Git panel.", "Git 패널의 각 파일 옆에 추가/삭제 변경 개수를 표시할지 여부입니다."),
    ('git_panel', 'show_count_badge', "Whether to show a badge on the git panel icon with the count of uncommitted changes.", "git 패널 아이콘에 커밋되지 않은 변경 개수 배지를 표시할지 여부입니다."),
    ('git_panel', 'scroll_bar', "How and when the scrollbar should be displayed.", "스크롤바를 표시할 방식과 시점입니다."),
    ('agent_panel', 'agent_panel_button', "Whether to show the agent panel button in the status bar.", "상태 표시줄에 에이전트 패널 버튼을 표시할지 여부입니다."),
    ('agent_panel', 'agent_panel_dock', "Where to dock the agent panel.", "에이전트 패널을 도킹할 위치입니다."),
    ('agent_panel', 'agent_panel_default_width', "Default width when the agent panel is docked to the left or right.", "에이전트 패널을 좌측 또는 우측에 도킹할 때의 기본 너비입니다."),
    ('agent_panel', 'agent_panel_default_height', "Default height when the agent panel is docked to the bottom.", "에이전트 패널을 하단에 도킹할 때의 기본 높이입니다."),

    # === 카테고리 11: editor_page (60) ===
    ('auto_save', 'auto_save_mode', "When to auto save buffer changes.", "버퍼 변경을 자동 저장할 시점입니다."),
    ('auto_save', 'delay_milliseconds', "Save after inactivity period (in milliseconds).", "비활성 시간(밀리초) 이후 저장합니다."),
    ('which_key', 'show_which_key_menu', "Display the which-key menu with matching bindings while a multi-stroke binding is pending.", "다중 스트로크 바인딩이 대기 중일 때 일치하는 바인딩과 함께 which-key 메뉴를 표시합니다."),
    ('which_key', 'menu_delay', "Delay in milliseconds before the which-key menu appears.", "which-key 메뉴가 나타나기까지의 지연 시간(밀리초)입니다."),
    ('multibuffer', 'double_click_in_multibuffer', "What to do when multibuffer is double-clicked in some of its excerpts.", "멀티버퍼의 발췌 부분을 더블 클릭할 때의 동작입니다."),
    ('multibuffer', 'expand_excerpt_lines', "How many lines to expand the multibuffer excerpts by default.", "멀티버퍼 발췌를 기본적으로 확장할 줄 수입니다."),
    ('multibuffer', 'excerpt_context_lines', "How many lines of context to provide in multibuffer excerpts by default.", "멀티버퍼 발췌에 기본적으로 제공할 컨텍스트 줄 수입니다."),
    ('multibuffer', 'expand_outlines_with_depth', "Default depth to expand outline items in the current file.", "현재 파일의 아웃라인 항목을 확장할 기본 깊이입니다."),
    ('multibuffer', 'diff_view_style', "How to display diffs in the editor.", "편집기에서 diff를 표시하는 방식입니다."),
    ('scrolling', 'scroll_beyond_last_line', "Whether the editor will scroll beyond the last line.", "편집기가 마지막 줄 이후로 스크롤할지 여부입니다."),
    ('scrolling', 'vertical_scroll_margin', "The number of lines to keep above/below the cursor when auto-scrolling.", "자동 스크롤 시 커서 위/아래에 유지할 줄 수입니다."),
    ('scrolling', 'horizontal_scroll_margin', "The number of characters to keep on either side when scrolling with the mouse.", "마우스로 스크롤할 때 양쪽에 유지할 글자 수입니다."),
    ('scrolling', 'scroll_sensitivity', "Scroll sensitivity multiplier for both horizontal and vertical scrolling.", "가로 및 세로 스크롤의 감도 배율입니다."),
    ('scrolling', 'fast_scroll_sensitivity', "Fast scroll sensitivity multiplier for both horizontal and vertical scrolling.", "가로 및 세로 빠른 스크롤의 감도 배율입니다."),
    ('scrolling', 'autoscroll_on_clicks', "Whether to scroll when clicking near the edge of the visible text area.", "보이는 텍스트 영역의 가장자리 근처를 클릭할 때 스크롤할지 여부입니다."),
    ('scrolling', 'sticky_scroll', "Whether to stick scopes to the top of the editor", "편집기 상단에 스코프를 고정할지 여부입니다."),
    ('signature_help', 'auto_signature_help', "Automatically show a signature help pop-up.", "시그니처 도움말 팝업을 자동으로 표시합니다."),
    ('signature_help', 'show_signature_help_after_edits', "Show the signature help pop-up after completions or bracket pairs are inserted.", "자동 완성 또는 괄호 쌍을 삽입한 후 시그니처 도움말 팝업을 표시합니다."),
    ('signature_help', 'snippet_sort_order', "Determines how snippets are sorted relative to other completion items.", "다른 자동 완성 항목 대비 스니펫의 정렬 방식을 결정합니다."),
    ('hover_popover', 'enabled', "Show the informational hover box when moving the mouse over symbols in the editor.", "편집기의 심볼 위로 마우스를 옮길 때 정보 호버 상자를 표시합니다."),
    ('hover_popover', 'delay', "Time to wait in milliseconds before showing the informational hover box.", "정보 호버 상자를 표시하기까지의 대기 시간(밀리초)입니다."),
    ('drag_and_drop_selection', 'enabled', "Enable drag and drop selection.", "드래그 앤 드롭 선택을 사용합니다."),
    ('drag_and_drop_selection', 'delay', "Delay in milliseconds before drag and drop selection starts.", "드래그 앤 드롭 선택이 시작되기까지의 지연 시간(밀리초)입니다."),
    ('gutter', 'show_line_numbers', "Show line numbers in the gutter.", "거터에 줄 번호를 표시합니다."),
    ('gutter', 'relative_line_numbers', 'Controls line number display in the editor\'s gutter. \\"disabled\\" shows absolute line numbers, \\"enabled\\" shows relative line numbers for each absolute line, and \\"wrapped\\" shows relative line numbers for every line, absolute or wrapped.', "편집기 거터의 줄 번호 표시를 제어합니다. \"disabled\"는 절대 줄 번호를, \"enabled\"는 각 절대 줄에 상대 줄 번호를, \"wrapped\"는 절대 또는 줄바꿈된 모든 줄에 상대 줄 번호를 표시합니다."),
    ('gutter', 'show_runnables', "Show runnable buttons in the gutter.", "거터에 실행 가능 버튼을 표시합니다."),
    ('gutter', 'show_folds', "Show code folding controls in the gutter.", "거터에 코드 접기 컨트롤을 표시합니다."),
    ('gutter', 'min_line_number_digits', "Minimum number of characters to reserve space for in the gutter.", "거터에 공간을 예약할 최소 글자 수입니다."),
    ('gutter', 'inline_code_actions', "Show code action button at start of buffer line.", "버퍼 줄의 시작 부분에 코드 액션 버튼을 표시합니다."),
    ('scrollbar', 'show', "When to show the scrollbar in the editor.", "편집기에서 스크롤바를 표시할 시점입니다."),
    ('scrollbar', 'cursors', "Show cursor positions in the scrollbar.", "스크롤바에 커서 위치를 표시합니다."),
    ('scrollbar', 'git_diff', "Show Git diff indicators in the scrollbar.", "스크롤바에 Git diff 표시기를 표시합니다."),
    ('scrollbar', 'search_results', "Show buffer search result indicators in the scrollbar.", "스크롤바에 버퍼 검색 결과 표시기를 표시합니다."),
    ('scrollbar', 'selected_text', "Show selected text occurrences in the scrollbar.", "스크롤바에 선택한 텍스트의 출현을 표시합니다."),
    ('scrollbar', 'selected_symbol', "Show selected symbol occurrences in the scrollbar.", "스크롤바에 선택한 심볼의 출현을 표시합니다."),
    ('scrollbar', 'diagnostics', "Which diagnostic indicators to show in the scrollbar.", "스크롤바에 표시할 진단 표시기 종류입니다."),
    ('scrollbar', 'horizontal_scrollbar', "When false, forcefully disables the horizontal scrollbar.", "false이면 가로 스크롤바를 강제로 비활성화합니다."),
    ('scrollbar', 'vertical_scrollbar', "When false, forcefully disables the vertical scrollbar.", "false이면 세로 스크롤바를 강제로 비활성화합니다."),
    ('minimap', 'show', "When to show the minimap in the editor.", "편집기에서 미니맵을 표시할 시점입니다."),
    ('minimap', 'display_in', "Where to show the minimap in the editor.", "편집기에서 미니맵을 표시할 위치입니다."),
    ('minimap', 'thumb', "When to show the minimap thumb.", "미니맵 썸을 표시할 시점입니다."),
    ('minimap', 'thumb_border', "Border style for the minimap's scrollbar thumb.", "미니맵 스크롤바 썸의 테두리 스타일입니다."),
    ('minimap', 'current_line_highlight', "How to highlight the current line in the minimap.", "미니맵에서 현재 라인을 강조하는 방식입니다."),
    ('minimap', 'max_width_columns', "Maximum number of columns to display in the minimap.", "미니맵에 표시할 최대 컬럼 수입니다."),
    ('toolbar', 'breadcrumbs', "Show breadcrumbs.", "경로 표시줄을 표시합니다."),
    ('toolbar', 'quick_actions', "Show quick action buttons (e.g., search, selection, editor controls, etc.).", "빠른 액션 버튼(검색, 선택, 편집기 컨트롤 등)을 표시합니다."),
    ('toolbar', 'selections_menu', "Show the selections menu in the editor toolbar.", "편집기 툴바에 선택 메뉴를 표시합니다."),
    ('toolbar', 'agent_review', "Show agent review buttons in the editor toolbar.", "편집기 툴바에 에이전트 검토 버튼을 표시합니다."),
    ('toolbar', 'code_actions', "Show code action buttons in the editor toolbar.", "편집기 툴바에 코드 액션 버튼을 표시합니다."),
    ('vim_settings', 'default_mode', "The default mode when Vim starts.", "Vim 시작 시 기본 모드입니다."),
    ('vim_settings', 'toggle_relative_line_numbers', "Toggle relative line numbers in Vim mode.", "Vim 모드에서 상대 줄 번호를 토글합니다."),
    ('vim_settings', 'use_system_clipboard', "Controls when to use system clipboard in Vim mode.", "Vim 모드에서 시스템 클립보드를 사용할 시점을 제어합니다."),
    ('vim_settings', 'use_smartcase_find', "Enable smartcase searching in Vim mode.", "Vim 모드에서 smartcase 찾기를 사용합니다."),
    ('vim_settings', 'global_substitution_default', "When enabled, the :substitute command replaces all matches in a line by default. The 'g' flag then toggles this behavior.", "활성화하면 :substitute 명령이 기본적으로 한 줄의 모든 일치 항목을 치환합니다. 그 후 'g' 플래그가 이 동작을 토글합니다."),
    ('vim_settings', 'highlight_on_yank_duration', "Duration in milliseconds to highlight yanked text in Vim mode.", "Vim 모드에서 yank한 텍스트를 강조할 시간(밀리초)입니다."),
    ('vim_settings', 'cursor_shape_normal_mode', "Cursor shape for normal mode.", "Normal 모드의 커서 모양입니다."),
    ('vim_settings', 'cursor_shape_insert_mode', "Cursor shape for insert mode. Inherit uses the editor's cursor shape.", "Insert 모드의 커서 모양입니다. Inherit는 편집기 커서 모양을 사용합니다."),
    ('vim_settings', 'cursor_shape_replace_mode', "Cursor shape for replace mode.", "Replace 모드의 커서 모양입니다."),
    ('vim_settings', 'cursor_shape_visual_mode', "Cursor shape for visual mode.", "Visual 모드의 커서 모양입니다."),
    ('vim_settings', 'custom_digraphs', "Custom digraph mappings for Vim mode.", "Vim 모드의 사용자 지정 디그래프 매핑입니다."),

    # === 카테고리 12: wallpaper_page (69, 사실 language settings 페이지) ===
    ('indentation', 'tab_size', "How many columns a tab should occupy.", "탭이 차지할 컬럼 수입니다."),
    ('indentation', 'hard_tabs', "Whether to indent lines using tab characters, as opposed to multiple spaces.", "여러 공백 대신 탭 문자로 줄을 들여쓸지 여부입니다."),
    ('indentation', 'auto_indent', "Controls automatic indentation behavior when typing.", "입력 시 자동 들여쓰기 동작을 제어합니다."),
    ('indentation', 'auto_indent_on_paste', "Whether indentation of pasted content should be adjusted based on the context.", "붙여넣은 내용의 들여쓰기를 컨텍스트에 따라 조정할지 여부입니다."),
    ('wrapping', 'soft_wrap', "How to soft-wrap long lines of text.", "긴 텍스트 줄의 소프트 랩 방식입니다."),
    ('wrapping', 'show_wrap_guides', "Show wrap guides in the editor.", "편집기에 줄바꿈 가이드를 표시합니다."),
    ('wrapping', 'preferred_line_length', "The column at which to soft-wrap lines, for buffers where soft-wrap is enabled.", "소프트 랩이 활성화된 버퍼에서 줄을 소프트 랩할 컬럼입니다."),
    ('wrapping', 'wrap_guides', "Character counts at which to show wrap guides in the editor.", "편집기에 줄바꿈 가이드를 표시할 글자 수입니다."),
    ('wrapping', 'allow_rewrap', "Controls where the `editor::rewrap` action is allowed for this language.", "이 언어에 대해 `editor::rewrap` 액션이 허용되는 위치를 제어합니다."),
    ('indent_guides', 'enabled', "Display indent guides in the editor.", "편집기에 들여쓰기 가이드를 표시합니다."),
    ('indent_guides', 'line_width', "The width of the indent guides in pixels, between 1 and 10.", "들여쓰기 가이드의 너비(픽셀)입니다(1~10)."),
    ('indent_guides', 'active_line_width', "The width of the active indent guide in pixels, between 1 and 10.", "활성 들여쓰기 가이드의 너비(픽셀)입니다(1~10)."),
    ('indent_guides', 'coloring', "Determines how indent guides are colored.", "들여쓰기 가이드의 색상 처리 방식을 결정합니다."),
    ('indent_guides', 'background_coloring', "Determines how indent guide backgrounds are colored.", "들여쓰기 가이드 배경의 색상 처리 방식을 결정합니다."),
    ('formatting', 'format_on_save', "Whether or not to perform a buffer format before saving.", "저장 전 버퍼 포매팅을 수행할지 여부입니다."),
    ('formatting', 'remove_trailing_whitespace_on_save', "Whether or not to remove any trailing whitespace from lines of a buffer before saving it.", "저장 전 버퍼 줄의 후행 공백을 제거할지 여부입니다."),
    ('formatting', 'ensure_final_newline_on_save', "Whether or not to ensure there's a single newline at the end of a buffer when saving it.", "저장 시 버퍼 끝에 단일 줄바꿈을 보장할지 여부입니다."),
    ('formatting', 'formatter', "How to perform a buffer format.", "버퍼 포매팅을 수행하는 방식입니다."),
    ('formatting', 'use_on_type_format', 'Whether to use additional LSP queries to format (and amend) the code after every \\"trigger\\" symbol input, defined by LSP server capabilities', "LSP 서버 기능에 정의된 \"트리거\" 심볼 입력 후 추가 LSP 쿼리로 코드를 포매팅(및 보정)할지 여부입니다."),
    ('formatting', 'code_actions_on_format', "Additional code actions to run when formatting.", "포매팅 시 실행할 추가 코드 액션입니다."),
    ('autoclose', 'use_autoclose', "Whether to automatically type closing characters for you. For example, when you type '(', Zed will automatically add a closing ')' at the correct position.", "닫는 문자를 자동으로 입력할지 여부입니다. 예를 들어 '('를 입력하면 올바른 위치에 닫는 ')'가 자동으로 추가됩니다."),
    ('autoclose', 'use_auto_surround', "Whether to automatically surround text with characters for you. For example, when you select text and type '(', Zed will automatically surround text with ().", "텍스트를 문자로 자동 둘러쌀지 여부입니다. 예를 들어 텍스트를 선택하고 '('를 입력하면 텍스트가 자동으로 ()로 둘러싸입니다."),
    ('autoclose', 'always_treat_brackets_as_autoclosed', "Controls whether the closing characters are always skipped over and auto-removed no matter how they were inserted.", "닫는 문자가 어떻게 삽입되었든 항상 건너뛰고 자동 제거할지 여부를 제어합니다."),
    ('autoclose', 'jsx_tag_auto_close', "Whether to automatically close JSX tags.", "JSX 태그를 자동으로 닫을지 여부입니다."),
    ('whitespace', 'show_whitespaces', "Whether to show tabs and spaces in the editor.", "편집기에 탭과 공백을 표시할지 여부입니다."),
    ('whitespace', 'space_whitespace_indicator', 'Visible character used to render space characters when show_whitespaces is enabled (default: \\"•\\")', "show_whitespaces가 활성화될 때 공백 문자를 렌더링하는 가시 문자입니다(기본값: \"•\")."),
    ('whitespace', 'tab_whitespace_indicator', 'Visible character used to render tab characters when show_whitespaces is enabled (default: \\"→\\")', "show_whitespaces가 활성화될 때 탭 문자를 렌더링하는 가시 문자입니다(기본값: \"→\")."),
    ('completions', 'show_completions_on_input', "Whether to pop the completions menu while typing in an editor without explicitly requesting it.", "편집기에서 명시적으로 요청하지 않아도 입력 중 자동 완성 메뉴를 띄울지 여부입니다."),
    ('completions', 'show_completion_documentation', "Whether to display inline and alongside documentation for items in the completions menu.", "자동 완성 메뉴의 항목 문서를 인라인 및 옆에 표시할지 여부입니다."),
    ('completions', 'words', "Controls how words are completed.", "단어 자동 완성 방식을 제어합니다."),
    ('completions', 'words_min_length', "How many characters has to be in the completions query to automatically show the words-based completions.", "단어 기반 자동 완성을 자동으로 표시하기 위해 자동 완성 쿼리에 필요한 최소 글자 수입니다."),
    ('completions', 'completion_menu_scrollbar', "When to show the scrollbar in the completion menu.", "자동 완성 메뉴에 스크롤바를 표시할 시점입니다."),
    ('completions', 'completion_detail_alignment', "Whether to align detail text in code completions context menus left or right.", "코드 자동 완성 컨텍스트 메뉴의 상세 텍스트를 왼쪽 또는 오른쪽으로 정렬할지 여부입니다."),
    ('inlay_hints', 'enabled', "Global switch to toggle hints on and off.", "힌트를 켜고 끄는 전역 스위치입니다."),
    ('inlay_hints', 'show_value_hints', "Global switch to toggle inline values on and off when debugging.", "디버깅 시 인라인 값을 켜고 끄는 전역 스위치입니다."),
    ('inlay_hints', 'show_type_hints', "Whether type hints should be shown.", "타입 힌트를 표시할지 여부입니다."),
    ('inlay_hints', 'show_parameter_hints', "Whether parameter hints should be shown.", "매개변수 힌트를 표시할지 여부입니다."),
    ('inlay_hints', 'show_other_hints', "Whether other hints should be shown.", "기타 힌트를 표시할지 여부입니다."),
    ('inlay_hints', 'show_background', "Show a background for inlay hints.", "인레이 힌트에 배경을 표시합니다."),
    ('inlay_hints', 'edit_debounce_ms', "Whether or not to debounce inlay hints updates after buffer edits (set to 0 to disable debouncing).", "버퍼 편집 후 인레이 힌트 업데이트를 디바운스할지 여부입니다(0으로 설정하면 디바운스 비활성화)."),
    ('inlay_hints', 'scroll_debounce_ms', "Whether or not to debounce inlay hints updates after buffer scrolls (set to 0 to disable debouncing).", "버퍼 스크롤 후 인레이 힌트 업데이트를 디바운스할지 여부입니다(0으로 설정하면 디바운스 비활성화)."),
    ('inlay_hints', 'toggle_on_modifiers_press', "Toggles inlay hints (hides or shows) when the user presses the modifiers specified.", "지정한 보조 키를 사용자가 누를 때 인레이 힌트를 토글(표시/숨김)합니다."),
    ('tasks', 'enabled', "Whether tasks are enabled for this language.", "이 언어에 대해 작업을 사용할지 여부입니다."),
    ('tasks', 'variables', "Extra task variables to set for a particular language.", "특정 언어에 설정할 추가 작업 변수입니다."),
    ('tasks', 'prefer_lsp', "Use LSP tasks over Zed language extension tasks.", "언어 익스텐션 작업보다 LSP 작업을 우선합니다."),
    ('miscellaneous', 'word_diff_enabled', "Whether to enable word diff highlighting in the editor. When enabled, changed words within modified lines are highlighted to show exactly what changed.", "편집기에서 단어 diff 강조를 활성화할지 여부입니다. 활성화하면 수정된 줄 안의 변경된 단어가 강조되어 무엇이 바뀌었는지 정확히 표시됩니다."),
    ('miscellaneous', 'middle_click_paste', "Enable middle-click paste on Linux.", "Linux에서 중간 클릭 붙여넣기를 사용합니다."),
    ('miscellaneous', 'extend_comment_on_newline', "Whether to start a new line with a comment when a previous line is a comment as well.", "이전 줄이 주석일 때 새 줄도 주석으로 시작할지 여부입니다."),
    ('miscellaneous', 'colorize_brackets', "Whether to colorize brackets in the editor.", "편집기에서 괄호를 색상화할지 여부입니다."),
    ('miscellaneous', 'vim_emacs_modeline_support', "Number of lines to search for modelines (set to 0 to disable).", "모드라인을 검색할 줄 수입니다(0으로 설정하면 비활성화)."),
    ('global_only_miscellaneous_sub', 'image_viewer', "The unit for image file sizes.", "이미지 파일 크기의 단위입니다."),
    ('global_only_miscellaneous_sub', 'auto_replace_emoji_shortcode', "Whether to automatically replace emoji shortcodes with emoji characters.", "이모지 단축코드를 이모지 문자로 자동 교체할지 여부입니다."),
    ('global_only_miscellaneous_sub', 'drop_size_target', "Relative size of the drop target in the editor that will open dropped file as a split pane.", "드롭한 파일을 분할 페인으로 열 편집기 내 드롭 대상의 상대 크기입니다."),
    ('global_only_miscellaneous_sub', 'lsp_document_colors', "How to render LSP color previews in the editor.", "편집기에서 LSP 색상 미리보기를 렌더링하는 방식입니다."),
    ('lsp', 'enable_language_server', "Whether to use language servers to provide code intelligence.", "코드 인텔리전스 제공에 언어 서버를 사용할지 여부입니다."),
    ('lsp', 'language_servers', "The list of language servers to use (or disable) for this language.", "이 언어에 대해 사용(또는 비활성화)할 언어 서버 목록입니다."),
    ('lsp', 'linked_edits', "Whether to perform linked edits of associated ranges, if the LS supports it. For example, when editing opening <html> tag, the contents of the closing </html> tag will be edited as well.", "LS가 지원하는 경우 관련 범위의 연결된 편집을 수행할지 여부입니다. 예를 들어 여는 <html> 태그를 편집하면 닫는 </html> 태그의 내용도 함께 편집됩니다."),
    ('lsp', 'go_to_definition_fallback', "Whether to follow-up empty Go to definition responses from the language server.", "언어 서버의 빈 정의로 이동 응답에 후속 처리를 할지 여부입니다."),
    ('lsp', 'lsp_folding_ranges', "When enabled, use folding ranges from the language server instead of indent-based folding.", "활성화하면 들여쓰기 기반 접기 대신 언어 서버의 접기 범위를 사용합니다."),
    ('lsp', 'lsp_document_symbols', "When enabled, use the language server's document symbols for outlines and breadcrumbs instead of tree-sitter.", "활성화하면 아웃라인 및 경로 표시에 tree-sitter 대신 언어 서버의 문서 심볼을 사용합니다."),
    ('lsp_completions', 'enabled', "Whether to fetch LSP completions or not.", "LSP 자동 완성을 가져올지 여부입니다."),
    ('lsp_completions', 'fetch_timeout_milliseconds', "When fetching LSP completions, determines how long to wait for a response of a particular server (set to 0 to wait indefinitely).", "LSP 자동 완성을 가져올 때 특정 서버의 응답을 기다릴 시간을 결정합니다(0으로 설정하면 무한 대기)."),
    ('lsp_completions', 'insert_mode', "Controls how LSP completions are inserted.", "LSP 자동 완성 삽입 방식을 제어합니다."),
    ('prettier', 'allowed', "Enables or disables formatting with Prettier for a given language.", "주어진 언어에 대해 Prettier 포매팅을 활성화 또는 비활성화합니다."),
    ('prettier', 'parser', "Forces Prettier integration to use a specific parser name when formatting files with the language.", "해당 언어로 파일을 포매팅할 때 Prettier 통합이 특정 파서 이름을 사용하도록 강제합니다."),
    ('prettier', 'plugins', "Forces Prettier integration to use specific plugins when formatting files with the language.", "해당 언어로 파일을 포매팅할 때 Prettier 통합이 특정 플러그인을 사용하도록 강제합니다."),
    ('prettier', 'options', "Default Prettier options, in the format as in package.json section for Prettier.", "기본 Prettier 옵션입니다. package.json의 Prettier 섹션과 같은 형식입니다."),
    ('prettier', 'show_edit_predictions', "Controls whether edit predictions are shown immediately or manually.", "편집 예측을 즉시 표시할지 수동으로 표시할지 제어합니다."),
    ('prettier', 'disable_in_language_scopes', "Controls whether edit predictions are shown in the given language scopes.", "주어진 언어 범위에서 편집 예측 표시 여부를 제어합니다."),
]

# Z3 적용된 영문 (en.json 키 값에 들어갈 깨끗한 영문) — Zed 언급 제거
EN_OVERRIDE = {
    ('workspace_restoration', 'restore_on_startup'):
        "What to restore from the previous session when opening the app.",
    ('git_integration', 'disable_git_integration'):
        "Disable all Git integration features.",
    ('general', 'disable_ai'):
        "Whether to disable all AI features.",
    ('file_scan', 'file_scan_exclusions'):
        "Files or globs of files that will be excluded entirely. They will be skipped during file scans, file searches, and not be displayed in the project file tree. Takes precedence over \"File Scan Inclusions\"",
    ('file_scan', 'file_scan_inclusions'):
        "Files or globs of files that will be included even when ignored by git. This is useful for files that are not tracked by git, but are still important to your project. Note that globs that are overly broad can slow down file scanning. \"File Scan Exclusions\" takes precedence over these inclusions",
    # 카테고리 9: appearance_page Zed 처리
    ('theme', 'icon_theme'):
        "The custom set of icons that will be associated with files and directories.",
    # 카테고리 12: wallpaper_page Zed 처리
    ('autoclose', 'use_autoclose'):
        "Whether to automatically type closing characters for you. For example, when you type '(', a closing ')' is automatically added at the correct position.",
    ('autoclose', 'use_auto_surround'):
        "Whether to automatically surround text with characters for you. For example, when you select text and type '(', the text is automatically surrounded with ().",
    ('tasks', 'prefer_lsp'):
        "Use LSP tasks over language extension tasks.",
}


def key_for(section: str, item: str) -> str:
    return f"settings_page.desc.{section}.{item}"


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "verify"
    root = Path(r"D:\Personal Project\Windows\Dokkaebi")

    if mode == "verify":
        # 키 충돌 + 중복 영문 확인
        keys = {}
        ens = {}
        for section, item, en, ko in ENTRIES:
            k = key_for(section, item)
            if k in keys:
                print(f"KEY COLLISION: {k}")
            keys[k] = (en, ko)
            ens.setdefault(en, []).append(k)
        for en, ks in ens.items():
            if len(ks) > 1:
                print(f"DUP EN: {en[:50]}... -> {ks}")
        print(f"OK: {len(ENTRIES)} entries, {len(keys)} unique keys")

    elif mode == "apply":
        en_path = root / "assets/locales/en.json"
        ko_path = root / "assets/locales/ko.json"
        rs_path = root / "crates/settings_ui/src/page_data.rs"

        en_lines, ko_lines, rs_replacements = [], [], []
        for section, item, en, ko in ENTRIES:
            k = key_for(section, item)
            en_value = EN_OVERRIDE.get((section, item), en)
            en_lines.append(f'  "{k}": {json.dumps(en_value, ensure_ascii=False)},')
            ko_lines.append(f'  "{k}": {json.dumps(ko, ensure_ascii=False)},')
            # page_data.rs 매칭에는 raw 영문(en) 사용
            old = f'description: "{en}"'
            new = f'description: "{k}"'
            rs_replacements.append((old, new))

        en_chunk = "\n".join(en_lines) + "\n"
        ko_chunk = "\n".join(ko_lines) + "\n"

        # en.json 수정 — 기존 settings_page.section.* 그룹 끝(wrapping) 다음에 추가
        en_text = en_path.read_text(encoding="utf-8")
        marker = '  "settings_page.section.wrapping": "Wrapping",\n'
        if marker not in en_text:
            print("ERROR: en.json marker not found"); sys.exit(1)
        en_text = en_text.replace(marker, marker + en_chunk, 1)
        en_path.write_text(en_text, encoding="utf-8")
        print(f"OK: en.json (+{len(en_lines)} keys)")

        # ko.json
        ko_text = ko_path.read_text(encoding="utf-8")
        marker = '  "settings_page.section.wrapping": "줄바꿈",\n'
        if marker not in ko_text:
            print("ERROR: ko.json marker not found"); sys.exit(1)
        ko_text = ko_text.replace(marker, marker + ko_chunk, 1)
        ko_path.write_text(ko_text, encoding="utf-8")
        print(f"OK: ko.json (+{len(ko_lines)} keys)")

        # page_data.rs
        rs_text = rs_path.read_text(encoding="utf-8")
        replaced, not_found = 0, []
        for old, new in rs_replacements:
            if old not in rs_text:
                not_found.append(old)
                continue
            count = rs_text.count(old)
            rs_text = rs_text.replace(old, new)
            replaced += count
        rs_path.write_text(rs_text, encoding="utf-8")
        print(f"OK: page_data.rs ({replaced} occurrences replaced)")
        if not_found:
            print(f"WARN: {len(not_found)} not found:")
            for n in not_found:
                print(f"  - {n[:80]}")


if __name__ == "__main__":
    main()
