# Dokkaebi shell integration (OSC 133 / FinalTerm)
# 자체 작성. FinalTerm 공개 사양 (https://wiki.gnome.org/Apps/Terminal/FinalTermPrototype) 만 참조.
# 다른 터미널/도구의 통합 스크립트는 참조하지 않았다.

# 인터랙티브 셸이 아니면 건너뛴다 (PTY 가 attach 되어 있어도 -c 모드 등에서 보호)
case "$-" in
    *i*) ;;
    *) return 0 ;;
esac

# 본 스크립트가 --rcfile 로 진입 시 ~/.bashrc 가 자동으로 로드되지 않으므로
# 사용자 .bashrc 를 한 번만 source 한다 (재진입 방지).
if [ -f "$HOME/.bashrc" ] && [ -z "$__DOKKAEBI_BASHRC_SOURCED" ]; then
    export __DOKKAEBI_BASHRC_SOURCED=1
    . "$HOME/.bashrc"
fi

__dokkaebi_running=0

# 프롬프트 직전(PROMPT_COMMAND): 직전 명령 종료(D) + 새 프롬프트 시작(A)
__dokkaebi_prompt_command() {
    local last_exit=$?
    __dokkaebi_running=1
    printf '\033]133;D;%s\033\\' "$last_exit"
    printf '\033]133;A\033\\'
    __dokkaebi_running=0
    return $last_exit
}

# 명령 실행 직전(DEBUG trap): C 마크 emit
# - 자동 완성 보조 호출(COMP_LINE) 이나 우리 자신의 함수 실행은 skip
__dokkaebi_preexec() {
    [ "$__dokkaebi_running" = "1" ] && return
    [ -n "$COMP_LINE" ] && return
    printf '\033]133;C\033\\'
}

if [ -n "$PROMPT_COMMAND" ]; then
    PROMPT_COMMAND="__dokkaebi_prompt_command; $PROMPT_COMMAND"
else
    PROMPT_COMMAND="__dokkaebi_prompt_command"
fi

trap '__dokkaebi_preexec' DEBUG
