# 공개 배포용 라이선스 정합성 보강

> **작성일**: 2026-04-29
> **상태**: 승인 대기

## 배경

공개 배포 전 라이선스 검토에서 다음 결손이 확인됨:
1. Inno Setup 인스톨러가 `LICENSE-GPL`/`LICENSE-APACHE`/`NOTICE`를 함께 배포하지 않음 → GPL §4·§5("수령자에게 라이선스 사본 전달") 위반 소지
2. 인스톨러 메타데이터에 `AppCopyright`/`VersionInfoCopyright` 누락
3. AGPL 라이선스 컴포넌트가 실제로는 0건이지만 루트 `LICENSE-AGPL` 파일과 NOTICE의 AGPL 언급, `crates/ztracing*/LICENSE-AGPL`이 잔재로 남아 메타·실태 불일치

아이콘(`welcome-icon*.png`/`app-icon-dokkaebi.ico`)은 사용자 확인 결과 자체 제작이라 작업 대상 아님.

## 목표

Dokkaebi의 GPL/Apache 의무를 인스톨러 배포 경로에서 정확히 충족시키고, AGPL 미사용 사실을 메타데이터에 일관되게 반영한다.

## AGPL 검토 결과 (확정 사실)

- 워크스페이스 모든 `Cargo.toml`에서 `license = "*AGPL*"` 표기 **0건** (Grep 검증 완료)
- `crates/ztracing/Cargo.toml:6`·`crates/ztracing_macro/Cargo.toml:6` 모두 `license = "GPL-3.0-or-later"`
- 메모리상 cargo-about 결과 13개 라이선스(Apache/MIT/MPL/Unicode/BSD/ISC/CC0/Zlib/MIT-0/NCSA/OpenSSL/bzip2)에 AGPL 부재
- 결론: **Dokkaebi는 AGPL 컴포넌트 미사용**. 루트 `LICENSE-AGPL` 파일과 ztracing 디렉터리의 `LICENSE-AGPL`은 상류 Zed에서 inherit된 잔재이며 현 시점 의무 발생 근거 없음

## 작업 항목

### A. 인스톨러에 라이선스 사본 동봉
- [x] `setup/dokkaebi.iss` `[Files]` 섹션에 다음 3개 파일을 `{app}\licenses\` 로 복사 추가
  - `LICENSE-GPL` → `licenses\LICENSE-GPL.txt` (확장자 부여로 메모장 더블클릭 열기 지원)
  - `LICENSE-APACHE` → `licenses\LICENSE-APACHE.txt`
  - `NOTICE` → `licenses\NOTICE.txt`
- [x] Source 경로는 setup 디렉터리 기준 상대경로(`{#ResourcesDir}\..\LICENSE-GPL` 등)로 작성. 빌드 파이프라인이 setup/에 미리 복사하지 않아도 동작하도록 `#ifexist` 가드는 두지 않음(라이선스 동봉은 의무이므로 부재 시 빌드 실패가 정상)
- [x] `assets/licenses.md`는 본체 바이너리에 임베드 + "오픈소스 라이선스" 메뉴로 접근 가능하므로 인스톨러 별도 동봉 불필요

### B. 인스톨러 저작권 메타데이터 추가
- [x] `setup/dokkaebi.iss` `[Setup]` 섹션에 `AppCopyright` 추가
  - 값: `Copyright (c) 2026 Dokkaebi. Based on Zed (c) 2022-2025 Zed Industries, Inc.`
  - Inno Setup이 `AppCopyright`를 자동으로 `VersionInfoCopyright`로도 사용하므로 별도 지정 불필요

### C. AGPL 잔재 정리 (실태 일치)
- [x] 루트 `LICENSE-AGPL` 파일 삭제
- [x] `crates/ztracing/LICENSE-AGPL` → `crates/ztracing/LICENSE-GPL`로 교체 (Cargo.toml 메타와 일치)
- [x] `crates/ztracing_macro/LICENSE-AGPL` → `crates/ztracing_macro/LICENSE-GPL`로 교체
- [x] `NOTICE` 본문에서 AGPL 언급 2곳 제거 + 표현 정정
  - "with some components licensed under the Apache License 2.0 (Apache-2.0) **or the GNU Affero General Public License, version 3 or later (AGPL-3.0-or-later)**" → AGPL 절 제거
  - "See LICENSE-GPL, **LICENSE-AGPL,** and LICENSE-APACHE" → AGPL 제거

## 검증 방법

- [x] **Inno Setup 컴파일 검증** (사용자 측): `setup/dokkaebi.iss` 컴파일 → `output\Dokkaebi-Setup-vX.Y.Z.exe` 생성 확인. 빌드 단계에서 `Source:` 경로 누락 시 컴파일 에러로 즉시 발견됨
- [x] **설치 후 검증** (사용자 측): 설치 폴더 `%LOCALAPPDATA%\Programs\Dokkaebi\licenses\`에 3개 파일 존재 + 메모장으로 열림 확인
- [x] **저작권 메타 검증** (사용자 측): 설치된 `dokkaebi.exe` 우클릭 → 속성 → 자세히 탭에서 Copyright 표기 확인 + Add/Remove Programs 게시자/저작권 확인
- [x] **빌드 검증**: AGPL 파일 삭제·교체는 코드 컴파일 영향 0이지만 안전 차원에서 `cargo check -p Dokkaebi` 1회 실행

## 승인 필요 항목

CLAUDE.md "승인 필요 조건"에 해당하는 작업:
- C. **파일 삭제**: 루트 `LICENSE-AGPL` 1건
- C. **파일 이름 변경(교체)**: `crates/ztracing/LICENSE-AGPL` → `LICENSE-GPL`, `crates/ztracing_macro/LICENSE-AGPL` → `LICENSE-GPL` 2건
- A. **외부 호환성에 가까운 변경**: 인스톨러 페이로드 추가(설치 폴더 구조에 `licenses\` 폴더 신설)

---

## 결정 필요 사항 (사용자 선택)

### 결정 1: 인스톨러 마법사에 라이선스 동의 페이지 추가 여부

- **옵션 1 (현 plan 채택)**: `LicenseFile=` 미사용. 라이선스 동의 페이지 표시하지 않고 설치 폴더에 사본만 동봉. → GPL §4·§5 의무는 "수령자에게 라이선스 사본 전달"이지 "동의 페이지 표시"가 아니므로 충족
- **옵션 2**: `LicenseFile=LICENSE-GPL` 추가 → 마법사에 GPL 전문 동의 페이지 표시. 사용자 경험상 무거워지나 더 보수적

### 결정 2: NOTICE에 Dokkaebi source URL 추가 여부

GPL §6은 바이너리 배포 시 source 위치 명시를 요구. 현재 인스톨러의 `AppPublisherURL=https://github.com/bitleader-dev/DokkaebiTerminal`이 그 역할이지만, NOTICE 본문에도 명시하면 더 명확.

- **옵션 1 (권장)**: NOTICE에 "Source code: https://github.com/bitleader-dev/DokkaebiTerminal" 1줄 추가
- **옵션 2**: 현 상태 유지 (인스톨러 메타로 충분하다고 판단)

위 결정 2건에 대한 답변 + plan 전체 승인 후 진행하겠습니다.
