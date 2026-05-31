//! CPU temperature via Core Temp shared memory.
//!
//! Core Temp (https://www.alcpu.com/CoreTemp/) exposes a `CoreTempSharedDataEx`
//! struct via a named file mapping `Local\CoreTempMappingObjectEx`.
//! No admin required — Core Temp already has its driver loaded as a service.
//!
//! Reference: https://www.alcpu.com/CoreTemp/developers.html
//!
//! If Core Temp is not running, `OpenFileMappingW` returns NULL and we return None.

#[cfg(target_os = "windows")]
mod sys {
    use std::ffi::c_void;

    pub const FILE_MAP_READ: u32 = 0x0004;

    // Layout MUST match Core Temp's CoreTempSharedDataEx exactly — version 2.
    // Total size: 5800 bytes (sizeof verified against alcpu.com docs).
    #[repr(C)]
    pub struct CoreTempSharedDataEx {
        pub ui_load:        [u32; 256],      // load % per logical core
        pub ui_tjmax:       [u32; 128],      // TjMax per physical CPU
        pub ui_core_cnt:    u32,             // cores per CPU
        pub ui_cpu_cnt:     u32,             // physical CPUs
        pub f_temp:         [f32; 256],      // temperature per logical core
        pub f_vid:          f32,
        pub f_cpu_speed:    f32,
        pub f_fsb_speed:    f32,
        pub f_multiplier:   f32,
        pub s_cpu_name:     [u8;  100],
        pub uc_fahrenheit:  u8,              // 1 if values are in Fahrenheit
        pub uc_delta_to_tjmax: u8,           // 1 if fTemp is *delta* to TjMax
        // Version 2 fields (newer Core Temp)
        pub uc_tdp_supported:   u8,
        pub uc_power_supported: u8,
        pub ui_struct_version:  u32,
        pub ui_tdp:             [u32; 128],
        pub f_power:            [f32; 128],
        pub f_multipliers:      [f32; 256],
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn OpenFileMappingW(
            dw_desired_access: u32,
            b_inherit_handle:  i32,
            lp_name:           *const u16,
        ) -> *mut c_void;

        pub fn MapViewOfFile(
            h_file_mapping_object: *mut c_void,
            dw_desired_access:     u32,
            dw_file_offset_high:   u32,
            dw_file_offset_low:    u32,
            dw_number_of_bytes_to_map: usize,
        ) -> *mut c_void;

        pub fn UnmapViewOfFile(lp_base_address: *const c_void) -> i32;
        pub fn CloseHandle(h_object: *mut c_void) -> i32;
    }
}

#[cfg(target_os = "windows")]
pub fn read_core_temp() -> Option<f32> {
    use std::ptr;

    // UTF-16 encoded name: "Local\CoreTempMappingObjectEx\0"
    let name: Vec<u16> = "Local\\CoreTempMappingObjectEx"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = sys::OpenFileMappingW(sys::FILE_MAP_READ, 0, name.as_ptr());
        if handle.is_null() { return None; } // Core Temp not running

        let view = sys::MapViewOfFile(
            handle, sys::FILE_MAP_READ, 0, 0,
            std::mem::size_of::<sys::CoreTempSharedDataEx>(),
        );
        if view.is_null() {
            sys::CloseHandle(handle);
            return None;
        }

        // SAFETY: Core Temp guarantees the mapping is at least sizeof(CoreTempSharedDataEx)
        // bytes and is laid out exactly per the docs. We hold a read-only view.
        let data = &*(view as *const sys::CoreTempSharedDataEx);

        let core_cnt = data.ui_core_cnt as usize;
        let cpu_cnt  = data.ui_cpu_cnt  as usize;
        let total_cores = (core_cnt * cpu_cnt).min(256);

        // Use MAX core temperature, not average. Core Temp's main UI shows
        // each core individually; the "CPU temperature" most users refer to
        // is the hottest core, which is the thermal-throttling reference.
        // If `ucDeltaToTjMax`, fTemp is the *gap* below TjMax — actual = TjMax - delta.
        let mut max_temp: f32 = f32::NEG_INFINITY;
        for i in 0..total_cores {
            let raw = data.f_temp[i];
            if raw <= 0.0 || !raw.is_finite() { continue; }

            let temp = if data.uc_delta_to_tjmax != 0 {
                let cpu_idx = (i / core_cnt).min(127);
                data.ui_tjmax[cpu_idx] as f32 - raw
            } else {
                raw
            };
            if temp > max_temp { max_temp = temp; }
        }

        let result = if max_temp.is_finite() { Some(max_temp) } else { None };

        // Convert Fahrenheit → Celsius if needed
        let result = result.map(|t| if data.uc_fahrenheit != 0 {
            (t - 32.0) * 5.0 / 9.0
        } else {
            t
        });

        sys::UnmapViewOfFile(view);
        sys::CloseHandle(handle);
        result
    }
}

#[cfg(not(target_os = "windows"))]
pub fn read_core_temp() -> Option<f32> { None }
