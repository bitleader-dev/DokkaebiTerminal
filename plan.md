# Zed v0.232.2 추가 백포트 (2026-04-16, 2차)

## 목표
직전 이식한 버그 fix 4건(#53114, #53359, #53126, #53264)에 **소규모 Features 2건** 추가 이식 후 커밋·푸시. 이어서 중규모 Features 5건 별도 plan으로 진행.

## 사전 조사 결과 (확정 사실)
- **#53103 SSH nickname**: `crates/remote/src/remote_client.rs:1285` `display_name()` match arm 중 `Ssh(opts)` → `opts.host.to_string()` 현재. 상류는 `opts.nickname.clone().unwrap_or_else(|| opts.host.to_string())`로 교체.
- **#52995 PastedImage name**: `crates/acp_thread/src/mention.rs` `MentionUri::PastedImage` variant가 unit 형태 → struct `PastedImage { name: String }`로 변경. 호출처 업데이트 필요 (agent/thread.rs, message_editor.rs, mention_set.rs, mention_crease.rs, thread_view.rs). 상류 patch의 모든 호출처 Dokkaebi에 존재 여부 확인 후 이식.

## 범위 (수정 대상)
### PR #53103 — SSH nickname 표시
- `crates/remote/src/remote_client.rs:1285-1288` match arm `Ssh(opts)` 내부 3줄 교체
- 테스트 `#[cfg(test)] mod tests`(+30줄) **생략**

### PR #52995 — PastedImage에 이름 필드 추가
- `crates/acp_thread/src/mention.rs` `MentionUri::PastedImage` → `PastedImage { name: String }`
- `MentionUri::parse_str` 분기에서 `name` query param 파싱
- `MentionUri::to_uri()` 등 직렬화 경로 업데이트
- 호출처: agent/thread.rs, agent_ui/message_editor.rs, mention_set.rs, ui/mention_crease.rs, conversation_view/thread_view.rs
- Dokkaebi 호출처에 `PastedImage` 패턴 매칭 있는지 grep로 전수 확인 후 모두 수정

## 수정 제외 (가드레일)
- 테스트 추가 생략
- visibility/시그너처 추가 변경 없음
- 중규모 Features 5건(#50582, #53033, #53194, #50221, #49881)은 본 plan **이후** 별도 plan으로 진행

## 작업 단계
- [ ] 1. PR #53103 이식 + `cargo check -p remote`
- [ ] 2. PR #52995 호출처 grep + 이식 + `cargo check -p acp_thread -p agent_ui -p agent`
- [ ] 3. 전체 `cargo check -p Dokkaebi` 최종 검증
- [ ] 4. `notes.md` 갱신
- [ ] 5. git commit + push
- [ ] 6. 중규모 5건 개별 plan.md 재작성 후 순차 진행

## 검증 방법
- 각 단계마다 빌드 통과 확인, exit 0, 신규 경고 0건

## 승인 필요 사항
- 사용자 "a" 선택으로 승인 완료
