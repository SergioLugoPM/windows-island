//! System statistics — CPU, RAM, network speed, battery, and (optional) CPU
//! temperature via LibreHardwareMonitor's HTTP server.
//!
//! Architecture notes:
//!  - We keep ONE persistent `sysinfo::System` across calls (in `Mutex<System>`),
//!    refresh just CPU + memory + networks on each tick. Constructing a fresh
//!    `System` per call would re-walk /proc and take ~50ms.
//!  - CPU usage requires two samples ~200ms apart for a meaningful number.
//!    We always have at least one prior sample because the manager runs every 1s.
//!  - Network speed = bytes since last refresh / time since last refresh.
//!  - Battery via `GetSystemPowerStatus` raw FFI — no extra crate needed.
//!  - CPU temperature: best-effort GET to http://localhost:8085/data.json
//!    (LHM default). Silent fail if not running.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SystemStats {
    /// 0..100 — overall CPU usage averaged across cores
    pub cpu_percent: f32,
    /// 0..100 — used RAM / total RAM
    pub ram_percent: f32,
    /// Total RAM in MiB (display only)
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    /// Aggregate network throughput in KiB/s since the last refresh
    pub net_down_kbps: f64,
    pub net_up_kbps: f64,
    /// 0..100, or -1 if no battery (desktop)
    pub battery_percent: i32,
    pub battery_charging: bool,
    /// CPU package temperature in °C; None if unavailable
    pub cpu_temp_c: Option<f32>,
}

pub struct StatsState {
    pub sys: Mutex<System>,
    pub networks: Mutex<Networks>,
    pub last_refresh: Mutex<Instant>,
}

impl StatsState {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(networks),
            last_refresh: Mutex::new(Instant::now()),
        }
    }
}

impl Default for StatsState {
    fn default() -> Self { Self::new() }
}

/// Build a SystemStats snapshot using the persistent System instance.
/// First call has a 200ms blocking pause to seed CPU deltas.
pub fn collect(state: &StatsState) -> SystemStats {
    let mut sys = state.sys.lock().unwrap();
    let mut networks = state.networks.lock().unwrap();
    let mut last = state.last_refresh.lock().unwrap();

    let now = Instant::now();
    let elapsed_s = now.duration_since(*last).as_secs_f64().max(0.001);
    *last = now;

    sys.refresh_cpu_usage();
    sys.refresh_memory();
    networks.refresh();

    let cpu_percent = sys.global_cpu_usage();

    let ram_total = sys.total_memory();
    let ram_used  = sys.used_memory();
    let ram_total_mb = ram_total / (1024 * 1024);
    let ram_used_mb  = ram_used  / (1024 * 1024);
    let ram_percent  = if ram_total > 0 {
        (ram_used as f32 / ram_total as f32) * 100.0
    } else { 0.0 };

    let (mut rx_bytes, mut tx_bytes) = (0u64, 0u64);
    for (_name, net) in networks.iter() {
        rx_bytes += net.received();
        tx_bytes += net.transmitted();
    }
    let net_down_kbps = (rx_bytes as f64 / elapsed_s) / 1024.0;
    let net_up_kbps   = (tx_bytes as f64 / elapsed_s) / 1024.0;

    let (battery_percent, battery_charging) = battery_status();
    let cpu_temp_c = read_cpu_temp_lhm();

    SystemStats {
        cpu_percent,
        ram_percent,
        ram_used_mb,
        ram_total_mb,
        net_down_kbps,
        net_up_kbps,
        battery_percent,
        battery_charging,
        cpu_temp_c,
    }
}

// ─── Battery via GetSystemPowerStatus ──────────────────────────────────────

#[cfg(windows)]
mod batt {
    #[repr(C)]
    pub struct SystemPowerStatus {
        pub ac_line_status:        u8,
        pub battery_flag:          u8,
        pub battery_life_percent:  u8,
        pub system_status_flag:    u8,
        pub battery_life_time:     u32,
        pub battery_full_life_time: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetSystemPowerStatus(lp_status: *mut SystemPowerStatus) -> i32;
    }
}

#[cfg(windows)]
fn battery_status() -> (i32, bool) {
    let mut s = batt::SystemPowerStatus {
        ac_line_status: 0, battery_flag: 0, battery_life_percent: 0,
        system_status_flag: 0, battery_life_time: 0, battery_full_life_time: 0,
    };
    let ok = unsafe { batt::GetSystemPowerStatus(&mut s) } != 0;
    if !ok { return (-1, false); }
    // battery_flag == 128 means "no system battery" (desktop)
    if s.battery_flag == 128 || s.battery_life_percent > 100 { return (-1, false); }
    let pct = s.battery_life_percent as i32;
    let charging = s.ac_line_status == 1;
    (pct, charging)
}

#[cfg(not(windows))]
fn battery_status() -> (i32, bool) { (-1, false) }

// ─── CPU temperature ────────────────────────────────────────────────────────
// Reads from Core Temp's shared memory (if Core Temp is running). Otherwise None.
// See cpu_temp.rs for the FFI details.

fn read_cpu_temp_lhm() -> Option<f32> {
    crate::cpu_temp::read_core_temp()
}
