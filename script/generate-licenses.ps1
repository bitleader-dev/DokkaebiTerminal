$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$CARGO_ABOUT_VERSION="0.8.2"
$outputFile=$args[0] ? $args[0] : "$(Get-Location)/assets/licenses.md"
$templateFile="script/licenses/template.md.hbs"

# cargo about 는 -o 로 파일을 덮어쓰므로, 먼저 cargo about 이 생성한 본문을 생성한 뒤
# 테마/아이콘/번들 라이선스 섹션을 파일 앞에 prepend 하는 순서로 처리한다.

$needsInstall = $false
try {
    $versionOutput = cargo about --version
    if (-not ($versionOutput -match "cargo-about $CARGO_ABOUT_VERSION")) {
        $needsInstall = $true
    } else {
        Write-Host "cargo-about@$CARGO_ABOUT_VERSION is already installed"
    }
} catch {
    $needsInstall = $true
}

if ($needsInstall) {
    Write-Host "Installing cargo-about@$CARGO_ABOUT_VERSION..."
    cargo install "cargo-about@$CARGO_ABOUT_VERSION"
}

Write-Host "Generating cargo licenses"

$failFlag = $env:ALLOW_MISSING_LICENSES ? "--fail" : ""
$args = @('about', 'generate', $failFlag, '-c', 'script/licenses/zed-licenses.toml', $templateFile, '-o', $outputFile) | Where-Object { $_ }
cargo @args

Write-Host "Prepending theme/icon/bundled license sections"
$headerLines = @()
$headerLines += "# ###### THEME LICENSES ######"
$headerLines += Get-Content assets/themes/LICENSES
$headerLines += ""
$headerLines += "# ###### ICON LICENSES ######"
$headerLines += Get-Content assets/icons/LICENSES
$headerLines += ""
$headerLines += Get-Content assets/bundled-licenses.md
$headerLines += ""
$headerLines += "# ###### CODE LICENSES ######"
$headerLines += ""

$cargoContent = Get-Content $outputFile
$combined = $headerLines + $cargoContent
$combined | Set-Content -Path $outputFile

Write-Host "Applying replacements"
$replacements = @{
    '&quot;' = '"'
    '&#x27;' = "'"
    '&#x3D;' = '='
    '&#x60;' = '`'
    '&lt;'   = '<'
    '&gt;'   = '>'
}
$content = Get-Content $outputFile
foreach ($find in $replacements.keys) {
    $content = $content -replace $find, $replacements[$find]
}
$content | Set-Content $outputFile

Write-Host "generate-licenses completed. See $outputFile"
