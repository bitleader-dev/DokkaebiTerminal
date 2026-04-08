/// Windows PDH API를 사용한 GPU 사용률 수집
use std::collections::HashMap;

use windows::core::PCWSTR;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

/// PDH 카운터 경로: GPU 엔진별 사용률 (Windows 10 1709+)
const GPU_UTILIZATION_COUNTER: &str = "\\GPU Engine(*)\\Utilization Percentage\0";

/// GPU 사용률을 PDH 카운터로 조회하는 구조체
pub struct GpuPdhQuery {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    /// PDH 결과 버퍼. u64 단위로 8바이트 정렬을 보장한다.
    /// PDH는 구조체 배열 + 문자열 데이터를 하나의 연속 버퍼에 기록하므로
    /// buffer_size(바이트) 전체를 확보해야 한다.
    buffer: Vec<u64>,
}

// SAFETY: PDH_HQUERY와 PDH_HCOUNTER는 단일 소유자(MetricsCollector)만 사용하며,
// 동시 접근이 발생하지 않는다. 백그라운드 스레드에서 생성→사용→해제 사이클이
// 모두 같은 태스크 내에서 순차적으로 수행된다.
unsafe impl Send for GpuPdhQuery {}

impl GpuPdhQuery {
    /// PDH 쿼리를 생성하고 GPU 엔진 카운터를 등록한다.
    /// 카운터가 없는 환경(GPU 미지원 등)에서는 None을 반환한다.
    pub fn new() -> Option<Self> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            let status = PdhOpenQueryW(PCWSTR::null(), 0, &mut query);
            if status != 0 {
                return None;
            }

            let counter_path: Vec<u16> = GPU_UTILIZATION_COUNTER.encode_utf16().collect();
            let mut counter = PDH_HCOUNTER::default();
            let status =
                PdhAddEnglishCounterW(query, PCWSTR(counter_path.as_ptr()), 0, &mut counter);
            if status != 0 {
                let _ = PdhCloseQuery(query);
                return None;
            }

            // 첫 번째 수집 (기준값 설정 — PDH는 두 번째 호출부터 유효한 값 반환)
            let _ = PdhCollectQueryData(query);

            Some(Self {
                query,
                counter,
                buffer: Vec::new(),
            })
        }
    }

    /// GPU 사용률을 수집한다.
    /// 작업 관리자와 동일한 방식: 엔진 타입별로 모든 프로세스의 사용률을 합산한 뒤
    /// 엔진 타입별 합산 값 중 최대값을 반환한다.
    pub fn collect(&mut self) -> Option<f32> {
        unsafe {
            let status = PdhCollectQueryData(self.query);
            if status != 0 {
                return None;
            }

            // 필요한 버퍼 크기(바이트) 조회
            let mut buffer_size = 0u32;
            let mut item_count = 0u32;
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                None,
            );

            if status != PDH_MORE_DATA || buffer_size == 0 {
                return None;
            }

            // buffer_size 바이트를 u64 단위로 확보 (8바이트 정렬 보장)
            let u64_count = (buffer_size as usize + 7) / 8;
            self.buffer.resize(u64_count, 0);
            let buf_ptr = self.buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;

            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                Some(buf_ptr),
            );

            if status != 0 {
                return None;
            }

            let items = std::slice::from_raw_parts(buf_ptr, item_count as usize);

            // 엔진 타입별 사용률 합산
            // 인스턴스 이름 형식: pid_<PID>_luid_..._phys_<N>_eng_<N>_engtype_<TYPE>
            let mut engine_sums: HashMap<EngineKey, f64> = HashMap::new();
            for item in items {
                if item.FmtValue.CStatus != PDH_CSTATUS_VALID_DATA {
                    continue;
                }
                let value = item.FmtValue.Anonymous.doubleValue;
                if value <= 0.0 {
                    continue;
                }
                let name = item.szName.to_string().unwrap_or_default();
                if let Some(key) = parse_engine_key(&name) {
                    *engine_sums.entry(key).or_default() += value;
                }
            }

            // 엔진 타입별 합산 값 중 최대값 선택
            let max_usage = engine_sums.values().copied().fold(0.0f64, f64::max);
            Some(max_usage.min(100.0) as f32)
        }
    }
}

/// 물리 어댑터 + 엔진 타입을 식별하는 키
#[derive(Hash, Eq, PartialEq)]
struct EngineKey {
    /// 물리 어댑터 번호 (phys_N)
    phys: u32,
    /// 엔진 타입 문자열 시작 위치 (engtype_ 이후)
    engtype_hash: u64,
}

/// 인스턴스 이름에서 phys와 engtype을 추출한다.
/// 형식: `pid_..._phys_0_eng_0_engtype_3D`
fn parse_engine_key(name: &str) -> Option<EngineKey> {
    let phys_pos = name.find("_phys_")?;
    let after_phys = &name[phys_pos + 6..];
    let phys_end = after_phys.find('_').unwrap_or(after_phys.len());
    let phys: u32 = after_phys[..phys_end].parse().ok()?;

    let engtype_pos = name.find("_engtype_")?;
    let engtype = &name[engtype_pos + 9..];
    // 간단한 해시로 문자열 비교 대신 정수 키 사용
    let engtype_hash = engtype
        .bytes()
        .fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));

    Some(EngineKey {
        phys,
        engtype_hash,
    })
}

impl Drop for GpuPdhQuery {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}
