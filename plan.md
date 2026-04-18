# Inno Setup 다운그레이드 검사 버그 수정 + 앱 목록 표시명 정리 (2026-04-18)

## 목표
1. **다운그레이드 검사 미작동 버그 수정**: `{#AppId}` 매크로의 `{` escape가 Pascal string literal에서 두 개의 중괄호로 평가되어 레지스트리 키 매칭 실패. 결과적으로 항상 "신규 설치"로 오판.
2. **Windows 앱 목록 DisplayName**: "Dokkaebi 0.2.0" → "Dokkaebi"로 변경. 버전은 `DisplayVersion` 필드(별도 줄)에만 표시.

## 근거
### 1번 버그
- `#define AppId "{{B8F4E2A1-...-3A4B}"`: `{{`은 `[Setup]` 평가 시 한 개의 `{`로 해석되도록 escape (Inno Setup constant 충돌 회피).
- Pascal `[Code]`에서 `'{#AppId}'`는 ISPP 텍스트 치환 → `'{{B8F4...}'`. Pascal single-quoted string은 `{{`를 escape하지 않으므로 결과 문자열 = `{{B8F4...}` (중괄호 2개).
- 실제 레지스트리 키는 `HKCU\...\Uninstall\{B8F4...}_is1`라 매칭 실패 → `RegQueryStringValue` False → `Exit; Result := True` (신규 설치 분기) → 검사 무력화.

### 2번 표시
- `setup/dokkaebi.iss:17` `AppVerName={#AppDisplayName} {#Version}` 지정 시 Inno Setup이 이를 Add/Remove Programs `DisplayName`으로 사용.
- `AppVerName` 미지정 시 `AppName`("Dokkaebi") 단독 사용. 버전은 `AppVersion={#Version}`이 `DisplayVersion`으로 별도 노출 (이미지 두 번째 줄 `0.2.0 | Dokkaebi | 2026-04-18`).

## 설계
### 1번 수정
- ISPP 매크로 분리:
  - `#define AppGuid "{B8F4E2A1-7C3D-4E5F-9A1B-6D8E0F2C3A4B}"` — escape 없이 raw GUID
  - `#define AppId "{" + AppGuid` — `[Setup]` AppId용 escape 형태 ("{" + "{B8F4...}" = "{{B8F4...}")
- Pascal 코드의 레지스트리 경로 조립: `'{#AppGuid}' + '_is1'` 또는 `'..\Uninstall\' + '{#AppGuid}' + '_is1'` 형태로 `AppGuid` 사용.
- 결과: Pascal string은 `{B8F4...}_is1` (중괄호 1개) → 레지스트리 매칭 성공.

### 2번 수정
- `AppVerName={#AppDisplayName} {#Version}` 라인을 **제거**. (정책상 단순 삭제로 충분, 다른 효과 없음)
- `AppName`은 그대로 "Dokkaebi" 유지.
- 영향 검토: `AppVerName`은 사용자 가시 표시명에만 영향. `AppId`/`AppVersion`/`UninstallString` 등 다른 키는 무관.

## 범위 (수정 대상 파일)
1. `setup/dokkaebi.iss`
   - `#define AppGuid "..."` 1줄 신규 추가 (line 1 부근)
   - `#define AppId` 정의 변경 (raw GUID + 한 글자 escape)
   - `[Setup]`에서 `AppVerName=...` 라인 1줄 제거
   - `[Code]` `InitializeSetup()`의 `RegKey` 조립을 `'{#AppGuid}'` 기반으로 변경

## 작업 단계
- [x] 1. `setup/dokkaebi.iss` `#define AppGuid` 추가 + `AppId` 재정의 (raw + escape 분리)
- [x] 2. `[Setup]` `AppVerName` 라인 제거
- [x] 3. `[Code]` `InitializeSetup` `RegKey` 문자열을 `'{#AppGuid}'` 기반으로 교체
- [x] 4. ISCC 재컴파일 → `Successful compile (84.625 sec)`, 신규 경고 0건
- [x] 5. 시나리오 점검(코드 리뷰): Pascal 치환 결과 `{B8F4...-3A4B}_is1` (중괄호 1개) ✓, [Setup] AppId 평가 결과 `{B8F4...-3A4B}` 변경 전후 동일 ✓
- [x] 6. `notes.md` 갱신, `release_notes.md` UI/UX 섹션에 표시명 항목 추가 (다운그레이드 보호 항목은 기존 보안 항목 그대로 유지 — 의미상 동일)
- [x] 7. 완료 보고 (사용자 환경 검증 완료 — 다운그레이드 차단/경고 동작, Add/Remove Programs 표시명 "Dokkaebi"로 표시)

## 검증
- ISCC 컴파일 통과.
- 시나리오 검증 (사용자 환경): regedit으로 `DisplayVersion`을 `9.9.9`로 임시 변경 후 `Dokkaebi-Setup-v0.2.0.exe` 실행 → 한글 경고 다이얼로그 표시. 검증 후 원복.
- Add/Remove Programs에서 항목명이 "Dokkaebi 0.2.0" → "Dokkaebi"로 변경 확인.

## 승인 필요 사항
- `AppId` 정의 방식 변경 — 단 매크로 평가 결과(`{B8F4...-3A4B}`)는 동일하므로 **AppId 값 자체는 불변**. 기존 설치본의 제거/업그레이드 호환성 영향 없음.
- `AppVerName` 제거 — 표시명 단순화. 기능 영향 없음.
- 의존성 추가 없음, 다른 코드 변경 없음.
