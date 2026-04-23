#!/usr/bin/env bash
# Claude Code hook → dokkaebi-cli.exe --notify-* IPC 브리지.
# 인자: 알림 종류 ("stop" | "idle" | "permission" | "subagent-start" | "subagent-stop")
# stdin: Claude Code hook payload (JSON)
# 동작:
#   1. cli 바이너리(dokkaebi-cli.exe) 탐색. DOKKAEBI_CLI 환경변수 → PATH → 기본 설치 경로.
#      cli.exe는 Dokkaebi 본체에 Named Pipe로 IPC URL을 전달하는 별도 클라이언트.
#      본체에 직접 --notify-kind를 보내면 본체의 clap이 unknown argument로 거부함.
#   2. Dokkaebi 본체 인스턴스 실행 중인지 확인 (꺼져 있으면 cli가 새 본체를 spawn하므로 skip).
#   3. stdin JSON payload 파싱:
#      - 공통: cwd, session_id, transcript_path
#      - stop: stop_hook_active (true면 재귀 호출이므로 중복 방지 skip),
#               transcript_path 에서 마지막 user 프롬프트 + 마지막 assistant 응답 추출
#      - idle: payload.message (Claude 가 띄운 원본 메시지)
#      - permission: tool_name + tool_input 의 command/file_path preview
#      - subagent-start(PreToolUse/Task): tool_input 의 subagent_type/description/prompt
#        + 안정적 id(hash) 생성. SubagentStop 쪽에서 동일 tool_input 으로 같은 id 재생성하여 매칭.
#      - subagent-stop(PostToolUse/Task): tool_input hash + tool_response(최종 응답) 전달
#   4. dokkaebi-cli.exe --notify-kind <kind> [--notify-cwd ...] --notify-pid ...
#      [--notify-prompt ...] [--notify-response ...]
#      [--notify-tool-name ...] [--notify-tool-preview ...]
#      [--notify-idle-summary ...]
#      [--notify-session-id ...] [--notify-transcript-path ...]
#      [--notify-subagent-id ...] [--notify-subagent-type ...]
#      [--notify-subagent-description ...] [--notify-subagent-prompt ...]
#      [--notify-subagent-result ...]
# 알림 제목/본문 문자열은 본체가 i18n 으로 UI 언어에 맞춰 생성하므로
# dispatch.sh 는 동적 내용만 전달한다.
# 실패해도 Claude Code UX 영향 없도록 항상 0 반환.

set +e

KIND="${1:-stop}"

# 진단용 디버그 로깅 — 기본 비활성. DOKKAEBI_NOTIFY_DEBUG 가 설정돼 있을 때만
# 기록한다. 경로를 명시(`=/path/to.log`)하면 그 위치로, 그 외 truthy(`1`/`true`)면
# `$TEMP/dokkaebi-notify.log` 또는 `/tmp/dokkaebi-notify.log` 로 기록.
DEBUG_LOG=""
case "${DOKKAEBI_NOTIFY_DEBUG:-}" in
  "" | "0" | "false" | "FALSE" | "False")
    ;;
  /* | [A-Za-z]:[/\\]*)
    DEBUG_LOG="$DOKKAEBI_NOTIFY_DEBUG"
    ;;
  *)
    DEBUG_LOG="${TEMP:-/tmp}/dokkaebi-notify.log"
    ;;
esac
dbg() {
  [ -n "$DEBUG_LOG" ] || return 0
  printf '[%s] [%s] %s\n' "$(date +'%Y-%m-%d %H:%M:%S')" "$KIND" "$*" >>"$DEBUG_LOG" 2>/dev/null
}
dbg "=== dispatch.sh entry === kind=$KIND pid=$$ ppid=${PPID:-?}"

# 긴 문자열을 지정 길이로 자르고 말줄임표를 붙인다.
# 사용: RESULT=$(truncate "$TEXT" 200)
truncate() {
  local s="$1" n="$2"
  if [ ${#s} -gt "$n" ]; then
    printf '%s...' "${s:0:$((n - 3))}"
  else
    printf '%s' "$s"
  fi
}

# cli 실행 파일 탐색 — 본체(dokkaebi.exe)가 아닌 별도 cli 바이너리를 사용해야 한다.
# 본체는 paths_or_urls 위주 인자만 파싱하므로 --notify-kind 전달 시 clap 에러 발생.
find_cli() {
  if [ -n "${DOKKAEBI_CLI:-}" ] && [ -x "$DOKKAEBI_CLI" ]; then
    echo "$DOKKAEBI_CLI"
    return 0
  fi
  if command -v dokkaebi-cli.exe >/dev/null 2>&1; then
    command -v dokkaebi-cli.exe
    return 0
  fi
  # Windows 기본 설치 경로 (LOCALAPPDATA) — 인스톨러가 본체와 같은 디렉터리에 배포
  local default_path="${LOCALAPPDATA:-$HOME/AppData/Local}/Programs/Dokkaebi/dokkaebi-cli.exe"
  if [ -x "$default_path" ]; then
    echo "$default_path"
    return 0
  fi
  return 1
}

# jq 실행 파일 탐색 — Claude Code 훅 payload JSON 파싱용.
# 1순위: 사용자 PATH 의 jq (winget/scoop/수동 설치).
# 2순위: Dokkaebi 인스톨러가 번들해 {app}\jq.exe 로 배치한 바이너리(MIT, jqlang).
#        MSYS 셸이 PATH 전파를 놓치거나 사용자가 jq 를 설치 안 했어도 동작 보장.
find_jq() {
  if command -v jq >/dev/null 2>&1; then
    command -v jq
    return 0
  fi
  local default_path="${LOCALAPPDATA:-$HOME/AppData/Local}/Programs/Dokkaebi/jq.exe"
  if [ -x "$default_path" ]; then
    echo "$default_path"
    return 0
  fi
  return 1
}

CLI=$(find_cli)
if [ -z "$CLI" ]; then
  dbg "cli not found — exiting"
  exit 0
fi
dbg "cli resolved: $CLI"

# Dokkaebi 본체가 실행 중인지 확인 (없으면 cli가 새 본체를 spawn하므로 skip).
# tasklist는 Windows 명령. Git Bash/MSYS는 `/FI` 같은 단일 슬래시 인자를
# Windows 경로(`C:/Program Files/Git/FI`)로 변환해 tasklist가 "잘못된 인수"로
# 거부한다. POSIX 스타일 대시 플래그(-FI)는 경로 변환을 우회하면서 cmd.exe
# 환경에서도 동일하게 동작한다.
if command -v tasklist >/dev/null 2>&1; then
  if ! tasklist -FI "IMAGENAME eq dokkaebi.exe" 2>/dev/null | grep -qi "dokkaebi.exe"; then
    dbg "dokkaebi.exe not running — exiting"
    exit 0
  fi
  dbg "dokkaebi.exe running"
fi

# stdin payload 읽기. 이후 여러 번 파싱에 재사용한다(stdin 은 한 번만 읽을 수 있음).
PAYLOAD=$(cat 2>/dev/null || true)
dbg "payload length=${#PAYLOAD}"
if [ -n "$DEBUG_LOG" ] && [ -n "$PAYLOAD" ]; then
  # payload 처음 1500자까지 기록 (Agent tool_input 의 subagent_type/description/prompt 확인용).
  dbg "payload head: ${PAYLOAD:0:1500}"
fi

# jq 실행 파일 경로 해석 (PATH 1순위, 번들 2순위). 이후 모든 jq 호출은 "$JQ_PATH"
# 를 사용하므로 PATH 전파 실패 환경에서도 번들 jq 로 자동 폴백된다.
JQ_PATH=$(find_jq)
if [ -n "$JQ_PATH" ]; then
  HAVE_JQ=1
  dbg "jq resolved: $JQ_PATH"
else
  HAVE_JQ=0
  dbg "jq not found (PATH 및 번들 위치 모두 부재) — 동적 필드 파싱 건너뜀"
fi

# 공통: cwd 추출 (jq 우선, 없으면 grep fallback)
CWD=""
if [ -n "$PAYLOAD" ]; then
  if [ "$HAVE_JQ" = "1" ]; then
    CWD=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '.cwd // empty' 2>/dev/null)
  else
    CWD=$(printf '%s' "$PAYLOAD" | grep -oE '"cwd"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"cwd"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
  fi
fi

# 공통: session_id, transcript_path 추출 (subagent 이벤트 매칭 + transcript tail 리더용).
# 다른 KIND 는 기존 동작 유지 위해 DYN_ARGS 에 추가하지 않고, subagent-* 에서만 전달한다.
# jq 가 없으면 grep/sed 로 폴백(두 필드 모두 최상위 문자열). transcript_path 는 Windows
# 경로 백슬래시가 JSON escape("\\")로 들어오므로 `\\` → `\` 로 복원한다.
SESSION_ID=""
TRANSCRIPT_PATH_COMMON=""
if [ -n "$PAYLOAD" ]; then
  if [ "$HAVE_JQ" = "1" ]; then
    COMMON_FIELDS=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '[(.session_id // ""), (.transcript_path // "")] | @tsv' 2>/dev/null)
    IFS=$'\t' read -r SESSION_ID TRANSCRIPT_PATH_COMMON <<< "$COMMON_FIELDS"
  else
    SESSION_ID=$(printf '%s' "$PAYLOAD" | grep -oE '"session_id"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
    TRANSCRIPT_PATH_COMMON=$(printf '%s' "$PAYLOAD" | grep -oE '"transcript_path"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"transcript_path"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
    # JSON escape(\\) → 실제 백슬래시로 복원 (Windows 경로 호환).
    TRANSCRIPT_PATH_COMMON=${TRANSCRIPT_PATH_COMMON//\\\\/\\}
  fi
fi

# Claude Code payload 의 tool_use_id 를 그대로 subagent id 로 사용한다.
# PreToolUse 와 PostToolUse 는 동일 tool 호출에 대해 같은 tool_use_id(`toolu_01...`)
# 를 발행하므로 별도 해시 없이 id 매칭이 안정적으로 성립. jq 유무와 무관하게
# grep/sed 로 최상위 문자열 필드에서 추출한다(tool_use_id 는 nested 아님).
extract_tool_use_id() {
  printf '%s' "$1" | grep -oE '"tool_use_id"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"tool_use_id"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/'
}

# KIND별 동적 필드 파싱. jq 없으면 동적 필드 전부 생략되고 본체가 default_body 로 폴백.
DYN_ARGS=()
case "$KIND" in
  stop)
    # stop_hook_active=true 면 재귀 호출이므로 중복 알림 방지를 위해 즉시 종료.
    # 동시에 transcript_path 도 같은 jq 호출에서 뽑아 fork 회수를 줄인다.
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      STOP_FIELDS=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '[(.stop_hook_active // false), (.transcript_path // "")] | @tsv' 2>/dev/null)
      IFS=$'\t' read -r STOP_HOOK_ACTIVE TRANSCRIPT_PATH <<< "$STOP_FIELDS"
      if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
        exit 0
      fi
      if [ -n "$TRANSCRIPT_PATH" ] && [ -f "$TRANSCRIPT_PATH" ]; then
        # Stop hook 은 transcript flush 보다 먼저 발화하므로 마지막 user/assistant
        # 블록이 파일에 쓰일 때까지 잠시 대기한다. 읽을 transcript 가 있을 때만
        # 지연하므로 jq 미설치/ transcript 부재 환경은 즉시 진행한다.
        sleep 0.3
        # 마지막 user 프롬프트 — tool_result 블록만 있는 user 메시지는 제외하고
        # 실제 text 블록(또는 plain string content)을 가진 메시지만 고른다.
        PROMPT=$("$JQ_PATH" -rs '
          [
            .[] | select(.type == "user") |
            if .message.content | type == "string" then .
            elif [.message.content[] | select(.type == "text")] | length > 0 then .
            else empty
            end
          ] | last |
          if .message.content | type == "array"
          then [.message.content[] | select(.type == "text") | .text] | join(" ")
          else .message.content // empty
          end
        ' "$TRANSCRIPT_PATH" 2>/dev/null)
        # 마지막 assistant 응답 text 블록 전체를 한 줄로 합친다.
        RESPONSE=$("$JQ_PATH" -rs '
          [.[] | select(.type == "assistant" and .message.content)] | last |
          [.message.content[] | select(.type == "text") | .text] | join(" ")
        ' "$TRANSCRIPT_PATH" 2>/dev/null)
        PROMPT=$(truncate "$PROMPT" 200)
        RESPONSE=$(truncate "$RESPONSE" 200)
        [ -n "$PROMPT" ] && DYN_ARGS+=("--notify-prompt" "$PROMPT")
        [ -n "$RESPONSE" ] && DYN_ARGS+=("--notify-response" "$RESPONSE")
      fi
    fi
    ;;
  idle)
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      IDLE_MSG=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '.message // empty' 2>/dev/null)
      [ -n "$IDLE_MSG" ] && DYN_ARGS+=("--notify-idle-summary" "$IDLE_MSG")
    fi
    ;;
  permission)
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      # tool_name 과 tool_input preview 를 한 번의 jq 호출로 TSV 추출.
      # tool_input 은 command(Bash) / file_path(Edit/Read/Write) 우선, 둘 다 없으면
      # tool_input 전체를 80자로 잘라 preview 로 사용한다.
      PERM_FIELDS=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '
        [
          (.tool_name // ""),
          ((.tool_input | if .command then .command
                          elif .file_path then .file_path
                          else (tostring | .[0:80]) end) // "")
        ] | @tsv
      ' 2>/dev/null)
      IFS=$'\t' read -r TOOL_NAME TOOL_PREVIEW <<< "$PERM_FIELDS"
      TOOL_PREVIEW=$(truncate "$TOOL_PREVIEW" 120)
      [ -n "$TOOL_NAME" ] && DYN_ARGS+=("--notify-tool-name" "$TOOL_NAME")
      [ -n "$TOOL_PREVIEW" ] && DYN_ARGS+=("--notify-tool-preview" "$TOOL_PREVIEW")
    fi
    ;;
  subagent-start|subagent-stop)
    # PreToolUse/PostToolUse — matcher 제거로 모든 도구에 발화하므로 tool_name 으로
    # 필터링해 Agent(서브에이전트) 도구만 트리거로 사용한다. 중첩 Tool 호출(Bash/Read/…)
    # 은 outer payload 의 tool_name 이 그 내부 도구 이름이라 자연스럽게 걸러진다.
    #
    # 성능 최적화: 이전에는 tool_name/tool_use_id/tool_input/tool_response 가 각각
    # 별도 jq 프로세스를 spawn 해 훅당 4~5회 fork 비용이 발생했다. Windows Git Bash
    # 에서 프로세스 spawn 비용이 커 체감 지연이 있었으므로 branch 별 jq 호출을 한 번에
    # @tsv 로 통합. jq 없을 때 폴백은 tool_name + tool_use_id 만 추출하고 메타데이터는 생략.
    TOOL_NAME=""; SUBAGENT_ID=""
    SUB_TYPE=""; SUB_DESC=""; SUB_PROMPT=""; SUB_RESULT=""
    if [ -n "$PAYLOAD" ] && [ "$HAVE_JQ" = "1" ]; then
      if [ "$KIND" = "subagent-start" ]; then
        JOINED=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '
          [
            (.tool_name // ""),
            (.tool_use_id // ""),
            ((.tool_input // {}).subagent_type // ""),
            ((.tool_input // {}).description // ""),
            ((.tool_input // {}).prompt // "")
          ] | @tsv
        ' 2>/dev/null)
        IFS=$'\t' read -r TOOL_NAME SUBAGENT_ID SUB_TYPE SUB_DESC SUB_PROMPT <<< "$JOINED"
      else
        JOINED=$(printf '%s' "$PAYLOAD" | "$JQ_PATH" -r '
          [
            (.tool_name // ""),
            (.tool_use_id // ""),
            (
              .tool_response |
              if type == "string" then .
              elif type == "array" then [.[] | if type == "object" and .text then .text else (tostring) end] | join(" ")
              elif type == "object" and .content then
                if .content | type == "array" then
                  [.content[] | if type == "object" and .text then .text else (tostring) end] | join(" ")
                else (.content | tostring) end
              else (tostring) end
            )
          ] | @tsv
        ' 2>/dev/null)
        IFS=$'\t' read -r TOOL_NAME SUBAGENT_ID SUB_RESULT <<< "$JOINED"
      fi
    else
      # jq 미설치: tool_name + tool_use_id 만 grep/sed 로 복원. 메타/결과 생략.
      TOOL_NAME=$(printf '%s' "$PAYLOAD" | grep -oE '"tool_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
      SUBAGENT_ID=$(extract_tool_use_id "$PAYLOAD")
    fi
    if [ "$TOOL_NAME" != "Agent" ]; then
      dbg "skip: tool_name='$TOOL_NAME' (not Agent)"
      exit 0
    fi
    dbg "tool_name=Agent — $KIND 처리 진입 id=$SUBAGENT_ID"
    [ -n "$SUBAGENT_ID" ] && DYN_ARGS+=("--notify-subagent-id" "$SUBAGENT_ID")
    if [ "$KIND" = "subagent-start" ]; then
      # 서브에이전트 뷰는 Editor 기반이라 길이 제약이 없다. argv 한계만 고려해 넉넉히.
      SUB_PROMPT=$(truncate "$SUB_PROMPT" 10000)
      SUB_DESC=$(truncate "$SUB_DESC" 2000)
      dbg "type=$SUB_TYPE desc=$SUB_DESC"
      [ -n "$SUB_TYPE" ] && DYN_ARGS+=("--notify-subagent-type" "$SUB_TYPE")
      [ -n "$SUB_DESC" ] && DYN_ARGS+=("--notify-subagent-description" "$SUB_DESC")
      [ -n "$SUB_PROMPT" ] && DYN_ARGS+=("--notify-subagent-prompt" "$SUB_PROMPT")
    else
      SUB_RESULT=$(truncate "$SUB_RESULT" 20000)
      [ -n "$SUB_RESULT" ] && DYN_ARGS+=("--notify-subagent-result" "$SUB_RESULT")
    fi
    [ -n "$SESSION_ID" ] && DYN_ARGS+=("--notify-session-id" "$SESSION_ID")
    [ -n "$TRANSCRIPT_PATH_COMMON" ] && DYN_ARGS+=("--notify-transcript-path" "$TRANSCRIPT_PATH_COMMON")
    ;;
esac

# Claude Code 프로세스 PID = 이 bash script의 부모. Dokkaebi 본체가 이 PID의
# parent chain을 따라가며 각 터미널의 shell PID와 일치하는 터미널을 정확히
# 찾아 dot 인디케이터/그룹 배지를 해당 탭 한 곳에만 표시한다. 같은 이름의
# 터미널이 여러 개이거나 같은 cwd에 여러 탭이 열린 환경에서도 정확한 탭을
# 식별할 수 있다.
#
# MSYS/Git Bash 주의: 일반 `$PPID` 는 MSYS 가상 PID이며 Windows 시스템 프로세스
# 테이블과 매칭되지 않는다. `/proc/PID/winpid` 가상 파일이 해당 프로세스의
# Win32 PID를 제공하므로 이를 우선 사용하고, 파일이 없으면 `$PPID` 로 폴백
# (네이티브 bash 등 MSYS가 아닌 환경 대응).
CLAUDE_PID=""
if [ -r "/proc/${PPID}/winpid" ]; then
  CLAUDE_PID=$(cat "/proc/${PPID}/winpid" 2>/dev/null)
fi
if [ -z "$CLAUDE_PID" ]; then
  CLAUDE_PID="${PPID:-0}"
fi

# cli 호출을 **동기 실행**한다(`&` 없음). 이유: Dokkaebi 본체는 cli가 전송한
# Toolhelp snapshot으로 parent chain을 따라 Claude가 실행 중인 터미널을 식별
# 하는데, background 실행 시 bash/dispatch.sh가 cli와 동시에 종료하면서
# 그 조상(Claude의 wrapper 등)이 snapshot 시점에 이미 exit해 chain이 중간에
# 끊긴다. 동기 실행하면 bash가 cli 종료까지 살아있어 전체 chain을 캡처할 수
# 있다. cli는 IPC handshake 후 즉시 종료하므로 Claude Code hook 응답 지연은
# 수십 ms 수준으로 무시 가능.
dbg "claude_pid=$CLAUDE_PID cwd=$CWD dyn_arg_count=${#DYN_ARGS[@]}"
dbg "invoking cli"
if [ -n "$CWD" ]; then
  "$CLI" --notify-kind "$KIND" --notify-cwd "$CWD" --notify-pid "$CLAUDE_PID" "${DYN_ARGS[@]}" >/dev/null 2>&1
  CLI_RC=$?
else
  "$CLI" --notify-kind "$KIND" --notify-pid "$CLAUDE_PID" "${DYN_ARGS[@]}" >/dev/null 2>&1
  CLI_RC=$?
fi
dbg "cli exit rc=$CLI_RC"

exit 0
