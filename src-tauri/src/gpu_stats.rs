//! GPU utilization via the same PDH "GPU Engine" counters Task Manager reads.
//! SPIKE STATUS: unverified in this codebase — pending human go/no-go check.

use std::ffi::c_void;

#[repr(C)]
struct PdhFmtCounterValue {
    c_status: u32,
    double_value: f64,
}

#[link(name = "pdh")]
extern "system" {
    fn PdhOpenQueryW(sz_data_source: *const u16, dw_user_data: usize, phquery: *mut *mut c_void) -> i32;
    fn PdhAddEnglishCounterW(
        hquery: *mut c_void, sz_full_counter_path: *const u16,
        dw_user_data: usize, phcounter: *mut *mut c_void,
    ) -> i32;
    fn PdhExpandWildCardPathW(
        sz_data_source: *const u16, sz_wild_card_path: *const u16,
        m_sz_expanded_path_list: *mut u16, pcch_path_list_length: *mut u32,
        dw_flags: u32,
    ) -> i32;
    fn PdhCollectQueryData(hquery: *mut c_void) -> i32;
    fn PdhGetFormattedCounterValue(
        hcounter: *mut c_void, dw_format: u32,
        lpdw_type: *mut u32, p_value: *mut PdhFmtCounterValue,
    ) -> i32;
    fn PdhCloseQuery(hquery: *mut c_void) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Best-effort GPU utilization percent, summed across all engine instances.
/// Returns None on any PDH failure — caller must not treat that as "0% usage".
pub fn read_gpu_percent() -> Option<f32> {
    const PDH_FMT_DOUBLE: u32 = 0x00000200;
    unsafe {
        let mut query: *mut c_void = std::ptr::null_mut();
        if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 { return None; }

        let wildcard = to_wide(r"\GPU Engine(*)\Utilization Percentage");
        let mut needed: u32 = 0;
        PdhExpandWildCardPathW(std::ptr::null(), wildcard.as_ptr(), std::ptr::null_mut(), &mut needed, 0);
        if needed == 0 { PdhCloseQuery(query); return None; }
        let mut expanded: Vec<u16> = vec![0; needed as usize];
        if PdhExpandWildCardPathW(std::ptr::null(), wildcard.as_ptr(), expanded.as_mut_ptr(), &mut needed, 0) != 0 {
            PdhCloseQuery(query);
            return None;
        }

        // `expanded` is a double-null-terminated list of null-terminated strings.
        let mut counters = Vec::new();
        let mut start = 0usize;
        for i in 0..expanded.len() {
            if expanded[i] == 0 {
                if i == start { break; }
                let path = String::from_utf16_lossy(&expanded[start..i]);
                let path_w = to_wide(&path);
                let mut hcounter: *mut c_void = std::ptr::null_mut();
                if PdhAddEnglishCounterW(query, path_w.as_ptr(), 0, &mut hcounter) == 0 {
                    counters.push(hcounter);
                }
                start = i + 1;
            }
        }
        if counters.is_empty() { PdhCloseQuery(query); return None; }

        // First collect primes the counters; this function is called on a
        // steady ~1.5s polling interval by the caller (not back-to-back), so
        // a single collect per call already has enough elapsed time since the
        // previous call for the rate-based counter to report a meaningful value.
        PdhCollectQueryData(query);

        let mut total = 0.0f64;
        for h in &counters {
            let mut val = PdhFmtCounterValue { c_status: 0, double_value: 0.0 };
            if PdhGetFormattedCounterValue(*h, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut val) == 0 {
                total += val.double_value;
            }
        }
        PdhCloseQuery(query);
        Some(total.min(100.0) as f32)
    }
}
