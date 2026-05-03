//! Toolhelp32 기반 프로세스 parent/children 관계 스냅샷.
//!
//! `view::ClaudeSubagentView` 의 "터미널로 점프" 버튼이 클릭됐을 때
//! `parent_pid`(서브에이전트를 호출한 dispatch.sh 의 부모 chain) 가 어느
//! TerminalView 의 shell_pid 와 일치하는지 양방향 (ancestor / descendants)
//! 으로 매칭하기 위해 사용한다.
//!
//! 동일 로직이 `crates/zed/src/zed/open_listener.rs::ProcessSnapshot` 에도
//! 있으나 `crates/zed` → `claude_subagent_view` 역방향 dep 회피를 위해
//! 본 모듈에 별도 구현을 둔다. 60 LOC 수준이라 동기화 부담 낮음.
//! 한쪽 변경 시 다른 쪽도 함께 검토할 것.

use std::collections::{HashMap, HashSet};

/// 프로세스 parent/children 관계 snapshot. `capture()` 1회 호출 후
/// `ancestors_of` / `descendants_of` 를 여러 번 사용 가능.
pub struct ProcessSnapshot {
    /// PID → parent PID 매핑. 루트 프로세스는 map 에서 누락되거나 0 으로 기록.
    parent_of: HashMap<u32, u32>,
    /// children_of 는 `descendants_of` 호출 시 최초 1회 lazy 빌드.
    children_of: std::cell::OnceCell<HashMap<u32, Vec<u32>>>,
}

impl ProcessSnapshot {
    /// Windows: Toolhelp32 snapshot 1회 캡처.
    /// 캡처 실패 시 빈 스냅샷 반환 (모든 매칭이 fail 로 떨어져 안전).
    #[cfg(windows)]
    pub fn capture() -> Self {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        };

        let mut parent_of: HashMap<u32, u32> = HashMap::new();
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return Self {
                    parent_of,
                    children_of: std::cell::OnceCell::new(),
                };
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if Process32FirstW(snap, &mut entry).is_ok() {
                loop {
                    parent_of.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                    if Process32NextW(snap, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
        }
        Self {
            parent_of,
            children_of: std::cell::OnceCell::new(),
        }
    }

    /// 비-Windows 환경 stub. Dokkaebi 는 Windows 전용이라 실 호출 경로는
    /// 모두 cfg(windows) 분기. 빌드 호환을 위해 빈 스냅샷 반환.
    #[cfg(not(windows))]
    pub fn capture() -> Self {
        Self {
            parent_of: HashMap::new(),
            children_of: std::cell::OnceCell::new(),
        }
    }

    /// `start` 자신부터 parent chain 을 따라 PID 벡터를 수집한다.
    /// 시스템 루트 도달, cycle 감지, 최대 깊이(64) 중 하나가 맞으면 중단.
    pub fn ancestors_of(&self, start: u32) -> Vec<u32> {
        let mut chain = Vec::with_capacity(8);
        let mut current = start;
        let mut guard = 0usize;
        loop {
            if guard > 64 {
                break;
            }
            guard += 1;
            chain.push(current);
            let Some(&parent) = self.parent_of.get(&current) else {
                break;
            };
            if parent == 0 || parent == current || chain.contains(&parent) {
                break;
            }
            current = parent;
        }
        chain
    }

    fn children_map(&self) -> &HashMap<u32, Vec<u32>> {
        self.children_of.get_or_init(|| {
            let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
            for (&child, &parent) in &self.parent_of {
                children.entry(parent).or_default().push(child);
            }
            children
        })
    }

    /// `root` 로부터 BFS 로 descendants 집합 수집. 최대 방문 1024 로 cycle 가드.
    pub fn descendants_of(&self, root: u32) -> HashSet<u32> {
        let children = self.children_map();
        let mut result = HashSet::new();
        let mut queue = vec![root];
        let mut guard = 0usize;
        while let Some(p) = queue.pop() {
            if guard > 1024 {
                break;
            }
            guard += 1;
            if !result.insert(p) {
                continue;
            }
            if let Some(kids) = children.get(&p) {
                queue.extend(kids.iter().copied());
            }
        }
        result
    }
}
