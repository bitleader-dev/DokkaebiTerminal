# 메모장 자동 저장 비동기화 (`fs::Fs` 부분 도입) — 계획

> **작성일**: 2026-04-25
> **대상**: `crates/notepad_panel/src/notepad_panel.rs`
> **배경**: `/simplify` 코드리뷰 보류 항목 #2. 현재 notepad_panel 은 `self.fs: Arc<dyn fs::Fs>` 필드를 보유하면서도 `std::fs::write/read_to_string/remove_file/remove_dir_all/create_dir_all` 동기 호출 사용. 코드베이스 다른 영속화 코드(`settings_store`, `assistant_text_thread`)는 `cx.spawn` 안에서 `fs.atomic_write(path, json).await` 패턴 일관 사용.
> **목적**: ① 키스트로크 자동 저장 시 corruption 방어(`atomic_write` 의 임시파일 + rename), ② 코드베이스 일관성, ③ UI 스레드 블로킹 위험 추가 감소.
> **비목적**: 마이그레이션·swap·on_release 의 동기 보장 흐름은 그대로 유지(앱 종료/모드 전환 race 방어).

---

## 1. 옵션 비교

### 옵션 A — Atomic Write 만 (std::fs 유지, fs::Fs 미도입)
- `std::fs::write` 를 임시파일 + rename 패턴으로 교체 (직접 atomic 흉내)
- async 도입 0, fs::Fs 도입 0
- **장점**: 단순, 동기 보장 흐름 변경 없음, corruption 방어 핵심 가치
- **단점**: 코드베이스 다른 곳들과 일관성 결여, FakeFs 테스트 주입 가치 미획득, 직접 작성한 atomic 패턴이 `fs::atomic_write` 의 플랫폼별 구현(Windows 의 `MoveFileEx` 등)을 재발명

### 옵션 B — Hybrid (디바운스 경로만 async, on_release/마이그레이션은 std::fs) ★ 권고
- 디바운스 자동 저장은 `cx.spawn` 안에서 `self.fs.atomic_write(path, json).await`
- `on_release` / `flush_pending_save` (swap·마이그레이션) 는 `std::fs::write` 동기 그대로 유지
- **장점**: 코드베이스 일관성(다른 영속화 코드와 같은 패턴), corruption 방어, 동기 보장 흐름 그대로
- **단점**: 디스크 I/O 경로 두 가지(async / sync) 공존 — 작은 복잡도 증가

### 옵션 C — 전면 async 화
- 모든 디스크 I/O `fs::Fs` 트레이트 사용
- `on_release` 는 best-effort `cx.spawn` (await 없음, 종료 race 위험)
- 마이그레이션 함수도 async 로 재구성
- **장점**: 완전 일관성
- **단점**: on_release race 위험(앱 종료 시 마지막 입력 유실 가능), 마이그레이션 락스텝 깨짐, 사용자 명시 동작(모드 전환 즉시성) 보장 어려움

**권고: 옵션 B**. 이유: corruption 방어와 일관성 가치를 가져가면서, 동기 보장 흐름은 데이터 정합성을 위해 유지. 안전성 최대화.

---

## 2. 옵션 B 상세 설계

### 2-1. 변경 범위

**디바운스 경로 (async + atomic_write)**:
- `schedule_save` 가 `DebouncedDelay::fire_new` 콜백 안에서 직접 async 흐름 작성
- 흐름:
  1. suppress_save 가드, current_save_path None 가드
  2. 에디터 텍스트 추출, hash 계산
  3. last_saved_hash 와 동일하면 skip (변경 감지 가드)
  4. JSON 직렬화 (실패 시 log::warn + 종료)
  5. `fs.atomic_write(save_path, json).await`
  6. 성공 시 `this.update(cx, |this, _| this.last_saved_hash = Some(hash))`
  7. 실패 시 log::warn (last_saved_hash 갱신 안 함 → 다음 디바운스에서 재시도)
- `DebouncedDelay::fire_new` 는 콜백이 `Task<()>` 반환 — `cx.spawn` 으로 감싸 반환

**동기 경로 (std::fs 그대로)**:
- `flush_save(&mut self, cx: &App)` — 그대로
- `flush_pending_save(&mut self, cx: &App)` — 그대로 (cancel + 동기 flush_save)
- `cx.on_release` — 그대로 (앱 종료 race 방어)
- `migrate_single_to_multi` / `migrate_multi_to_single` 안의 `Self::write_to_file` — 그대로

**삭제 경로 (그대로 std::fs)**:
- `handle_workspace_notify` 의 사라진 그룹 파일 삭제 — std::fs::remove_file 그대로
- `migrate_multi_to_single` 의 `notepad/` 디렉터리 삭제 — std::fs::remove_dir_all 그대로
- 이유: 이 경로들은 마이그레이션·삭제 락스텝의 일부라 동기 보장 필요

### 2-2. 코드 형태 (예상)

```rust
fn schedule_save(&mut self, cx: &mut Context<Self>) {
    if self.suppress_save {
        return;
    }
    self.save_debouncer.fire_new(SAVE_DEBOUNCE, cx, |this, cx| {
        // 동기 가드 + 직렬화는 main thread 에서
        if this.suppress_save {
            return Task::ready(());
        }
        let Some(save_path) = this.current_save_path() else {
            return Task::ready(());
        };
        let text = this.editor.read(cx).text(cx);
        let hash = text_hash_for_disk(&text);
        if Some(hash) == this.last_saved_hash {
            return Task::ready(());
        }
        let trimmed = text.trim_end_matches(|c: char| c == '\n' || c == '\r');
        let data = NotepadData { content: trimmed.to_string() };
        let json = match serde_json::to_string_pretty(&data) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("notepad_panel: JSON 직렬화 실패 {:?}: {}", save_path, e);
                return Task::ready(());
            }
        };
        let fs = this.fs.clone();
        cx.spawn(async move |this, cx| {
            if let Err(e) = fs.atomic_write(save_path.clone(), json).await {
                log::warn!("notepad_panel: 비동기 쓰기 실패 {:?}: {}", save_path, e);
                return;
            }
            this.update(cx, |this, _cx| {
                this.last_saved_hash = Some(hash);
            })
            .ok();
        })
    });
}
```

### 2-3. 데이터 정합성 시나리오 검증

- **A. 디바운스 fire 중 사용자가 추가 입력**: 새 입력은 `BufferEdited` → `schedule_save` 호출 → DebouncedDelay 가 이전 fire 를 oneshot 으로 cancel + 새 timer 등록. 이전 atomic_write 가 이미 시작됐으면 끝까지 진행, last_saved_hash 갱신은 update 콜백이 entity 업데이트 시점에 적용. 이후 다음 fire 가 새 hash 로 진행.
- **B. 디바운스 fire 중 그룹 swap**: `handle_workspace_notify` 가 `flush_pending_save` 호출 → DebouncedDelay::cancel → pending 디바운스 즉시 취소. atomic_write 가 이미 fire 됐으면 끝까지 가서 옛 그룹 파일에 저장(정상). swap 본체는 동기 std::fs 로 옛 그룹에 또 저장 — 동일 콘텐츠라 idempotent.
- **C. 디바운스 fire 중 앱 종료**: `cx.on_release` 가 동기 flush_save 호출(std::fs). pending atomic_write 는 task drop 으로 cancel(가능) 또는 끝까지 fire(가능). 어느 쪽이든 마지막 텍스트는 on_release 동기 flush 가 보장.
- **D. atomic_write 실패**: log::warn 만 남기고 last_saved_hash 미갱신 → 다음 디바운스 fire 시 동일 hash 체크에서 갱신 안 된 hash 라 재시도. 단 사용자가 종료하면 손실 → on_release 동기 flush 가 폴백.
- **E. last_saved_hash race**: atomic_write 완료 후 update 콜백이 hash 갱신 시점에, 사용자가 그 사이 더 입력해 두 번째 fire 가 또 진행 중일 수 있음. 두 fire 의 hash 가 다르므로 두 번째 fire 의 hash 가 update 시 덮어씀 — 정상.

### 2-4. last_saved_hash 가 디스크와 어긋나는 경우

- 시나리오 D 의 atomic_write 실패 → hash 미갱신. 다음 fire 가 같은 텍스트면 hash 동일 → skip. 디스크와 메모리 hash 불일치 지속.
- 해결: 실패 시 `last_saved_hash = None` 으로 명시 무효화 → 다음 fire 가 무조건 재시도.
- 단점: 사용자가 입력을 멈추면 다음 fire 가 안 옴 → 계속 미저장. 그러나 swap·종료 시 동기 flush 가 폴백.
- **결정**: 실패 시 `last_saved_hash = None` 으로 무효화 (재시도 유리).

---

## 3. 변경 파일 목록

| 파일 | 수정 내용 |
|---|---|
| `crates/notepad_panel/src/notepad_panel.rs` | (1) `schedule_save` 본체 교체 — fire_new 콜백 안에서 동기 가드 + 직렬화 → `cx.spawn` 안에서 `fs.atomic_write().await` + 성공 시 hash 갱신, 실패 시 `last_saved_hash = None`. (2) `Task::ready` import 그대로 활용. (3) 동기 `flush_save` / `flush_pending_save` / `write_to_file` / `on_release` 클로저 그대로 유지. |
| `notes.md` | 2026-04-25 항목으로 변경 내역 추가 |

`release_notes.md`: 미반영. 사용자 체감 변화는 (a) atomic_write 으로 corruption 방어가 강화되었으나 평상시에 사용자가 인지할 변화 없음, (b) 평상시 키스트로크 자동 저장의 디스크 I/O 가 백그라운드로 이동해 UI 스레드 블로킹 추가 감소(이미 디바운스로 대부분 해소된 상태)라 별도 항목 분리 안 함. CLAUDE.md "내부 리팩토링·성능 개선" 기준에 부합.

---

## 4. 작업 단계

### Phase A — 코드 변경
- [x] **1. 범위 확인 — 승인 완료 (2026-04-25)**
- [x] 2. `notepad_panel.rs` 의 `schedule_save` 본체를 옵션 B 형태로 교체 (fire_new 콜백 → 동기 가드/직렬화 → cx.spawn 안 fs.create_dir + fs.atomic_write)
- [x] 3. atomic_write 실패 시 `last_saved_hash = None` 무효화 추가 (디렉터리 생성 실패 시도 동일)
- [x] 4. 동기 경로(flush_save / flush_pending_save / on_release / migrate_*) 변경 없음 재확인

### Phase B — 검증
- [x] 5. `cargo check -p notepad_panel` 통과 확인 (1.77s 증분, 신규 경고 0)
- [x] 6. 코드 리뷰 — 데이터 정합성 시나리오 A~E 한 번 더 트레이스 (DebouncedDelay::fire_new 의 previous_task.await 직렬화로 두 fire 가 동시 진행 불가 → race 없음 확인)
- [x] 7. 런타임 검증 (사용자 수동 — 2026-04-25 "잘됨" 확인)

### Phase C — 문서
- [x] 8. `notes.md` 항목 추가
- [x] 9. 완료 보고

> 검증 단계의 `[x]` 는 빌드/테스트 통과 후에만 표시한다.

---

## 5. 검증 방법

### 빌드 검증
- `cargo check -p notepad_panel` — 신규 경고 0 건
- `cargo check -p Dokkaebi` (lib) — 신규 경고·에러 0 건

### 런타임 검증 (사용자 수동)
1. **기본 자동 저장**: 메모 입력 → 300ms 대기 → 입력 멈춤 → `data_dir()/notepad.json` 또는 `notepad/<uuid>.json` 의 content 가 갱신되는지 확인.
2. **그룹 swap 시 즉시 저장**: 그룹 A 에 텍스트 입력 → 즉시(300ms 안에) 그룹 B 로 전환 → 그룹 A 의 파일에 저장되어 있는지 확인. (동기 flush_pending_save 가 폴백)
3. **앱 종료 시 즉시 저장**: 메모 입력 직후(300ms 안에) 앱 닫기 → 다음 실행 시 콘텐츠 복원되는지 확인. (on_release 동기 flush 폴백)
4. **모드 전환 마이그레이션**: 단일↔멀티 토글 시 데이터 손실 없이 정상 마이그레이션되는지 확인.
5. **연속 키스트로크**: 빠르게 타이핑(SAVE_DEBOUNCE 미만 간격으로) → 디바운스로 마지막 1 회만 저장되는지 + UI 프리징 없는지 확인.

---

## 6. 승인 필요 항목

CLAUDE.md "절대 금지 — 승인 필요":
1. **동작 변경 (블로킹 → 비동기)**: 키스트로크 자동 저장 경로가 main thread std::fs::write 동기 → bg task atomic_write async 로 변경. UI 응답성 개선이 의도이나 데이터 정합성 시나리오 A~E 검토 필요.
2. **옵션 B 채택**: 디바운스 경로만 async, 동기 보장 흐름(on_release/마이그레이션/swap)은 std::fs 유지. 옵션 A(전부 std::fs + 직접 atomic) 또는 옵션 C(전부 async)를 선호하면 그쪽으로 변경.
3. **atomic_write 실패 시 hash 무효화**: 실패 시 `last_saved_hash = None` → 다음 디바운스 재시도. 알트: 실패 시 무한 재시도 task spawn (비권장 — 폭주 위험).

승인되면 Phase A 부터 구현 진행하겠습니다.
