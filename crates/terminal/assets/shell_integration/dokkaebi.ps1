# Dokkaebi shell integration (OSC 133 / FinalTerm) for PowerShell
# 자체 작성. FinalTerm 공개 사양만 참조.

# -Command 진입 시 PowerShell 표준 시작 배너가 생략되므로 1회만 명시 출력.
# (pwsh 단독 실행 시 자동으로 보이는 "PowerShell <version>" 줄을 복원)
if (-not $global:__DokkaebiBannerShown) {
    $global:__DokkaebiBannerShown = $true
    Write-Host "PowerShell $($PSVersionTable.PSVersion)"
}

# -NoExit -Command 진입 시 $PROFILE 이 자동 로드되지 않으므로 1회만 명시 source.
if (-not $global:__DokkaebiProfileLoaded) {
    $global:__DokkaebiProfileLoaded = $true
    if ($PROFILE -and (Test-Path -LiteralPath $PROFILE)) {
        try { . $PROFILE } catch {
            Write-Verbose "Dokkaebi: failed to source profile: $_"
        }
    }
}

# 원본 prompt 함수 1회만 보존 (재진입/세션 재실행 시 중복 wrapping 방지).
if (-not $global:__DokkaebiOriginalPrompt) {
    $global:__DokkaebiOriginalPrompt = $function:prompt
}

# 새 prompt: 직전 명령 종료(D) + 프롬프트 시작(A) + 원본 prompt + 입력 영역(B).
function global:prompt {
    $exitCode = if ($?) { 0 } elseif ($LASTEXITCODE) { $LASTEXITCODE } else { 1 }
    $original = & $global:__DokkaebiOriginalPrompt
    "`e]133;D;$exitCode`e\`e]133;A`e\$original`e]133;B`e\"
}

# PSReadLine 가 있으면 Enter 키 hook 으로 명령 실행 시작(C) emit.
# PSReadLine 미설치/미지원 PowerShell 버전이면 D/A/B 만으로도 종료 코드 추적 가능.
if (Get-Module -ListAvailable -Name PSReadLine) {
    try {
        Set-PSReadLineKeyHandler -Key Enter -ScriptBlock {
            [Console]::Write("`e]133;C`e\")
            [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
        } -ErrorAction Stop
    } catch {
        Write-Verbose "Dokkaebi: PSReadLine Enter hook 등록 실패 — D/A/B 만 활성"
    }
}
