# wasmtime 33 → 36 업그레이드 재시도 (2026-04-16)

## 목표
Dependabot 보안 경고(critical 2, moderate 10, low 3, rand low 1) 해결을 위해 `wasmtime 33.0.2` → `wasmtime 36.0.7+`.

## 사전 조사 결과
- Zed 상류 main은 이미 v0.232부터 wasmtime 36 사용 중(v0.231까지 33)
- 이전 세션 메모리 "Zed 업스트림 동기화까지 대기" 전제 조건 **충족**
- 상류 Cargo.toml의 wasmtime features는 Dokkaebi와 동일 (async, demangle, runtime, cranelift, component-model, incremental-cache, parallel-compilation)
- 2026-04-14 실패 원인: `extension_host 761` 에러 — 구체적 원인 불명. 빌드 에러 로그 기반 대응

## 전략
1. `Cargo.toml`의 `wasmtime` / `wasmtime-wasi` 버전 `"33"` → `"36"`
2. `cargo check -p Dokkaebi` 시도 → 에러 분석
3. 에러 지점이 extension_host/host impl 쪽이면 상류 해당 파일과 비교해 API 변경 이식
4. 빌드 성공 후 `cargo check --all-targets` 추가 검증
5. 실패 지속 시 Cargo.toml 원복 후 사용자 보고

## 범위 (수정 대상 파일)
- `Cargo.toml` (2줄)
- `Cargo.lock` (자동 재생성)
- 빌드 에러 발생 시 `crates/extension_host/src/*.rs` (상류 대응)
- 기타 wasmtime API 사용처 (`crates/extension/*`, `crates/wasm_host/*` 등)

## 수정 제외 (가드레일)
- 기능 변경 없음, 의존성 버전업만
- 실패 시 **즉시 원복**, 추가 시도 중단 후 사용자 승인 재요청

## 작업 단계
- [ ] 1. Cargo.toml 버전 변경
- [ ] 2. `cargo check -p Dokkaebi` 1차 시도
- [ ] 3. 에러 로그 분석 + 상류 코드 대조
- [ ] 4. 필요 시 extension_host 등 수정
- [ ] 5. 재빌드 → 성공 시 `cargo check --all-targets`
- [ ] 6. rand 0.8.5 제거 확인 (`cargo tree -i rand:0.8.5`)
- [ ] 7. notes.md 갱신 + commit + push

## 승인 필요 사항
- 사용자 "재시도" 승인 완료
