use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(windows)]
mod win_metrics {
    use std::ffi::c_void;

    pub const TH32CS_SNAPTHREAD: u32 = 0x00000004;
    pub const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct PROCESS_MEMORY_COUNTERS {
        pub cb: u32,
        pub PageFaultCount: u32,
        pub PeakWorkingSetSize: usize,
        pub WorkingSetSize: usize,
        pub QuotaPeakPagedPoolUsage: usize,
        pub QuotaPagedPoolUsage: usize,
        pub QuotaPeakNonPagedPoolUsage: usize,
        pub QuotaNonPagedPoolUsage: usize,
        pub PagefileUsage: usize,
        pub PeakPagefileUsage: usize,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    #[derive(Default)]
    pub struct FILETIME {
        pub dwLowDateTime: u32,
        pub dwHighDateTime: u32,
    }

    impl FILETIME {
        pub fn to_u64(&self) -> u64 {
            ((self.dwHighDateTime as u64) << 32) | (self.dwLowDateTime as u64)
        }
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct THREADENTRY32 {
        pub dwSize: u32,
        pub cntUsage: u32,
        pub th32ThreadID: u32,
        pub th32OwnerProcessID: u32,
        pub tpBasePri: i32,
        pub tpDeltaPri: i32,
        pub dwFlags: u32,
    }

    extern "system" {
        pub fn GetCurrentProcess() -> *mut c_void;
        pub fn GetCurrentProcessId() -> u32;
        pub fn GetProcessTimes(
            hProcess: *mut c_void,
            lpCreationTime: *mut FILETIME,
            lpExitTime: *mut FILETIME,
            lpKernelTime: *mut FILETIME,
            lpUserTime: *mut FILETIME,
        ) -> i32;
        pub fn GetProcessHandleCount(hProcess: *mut c_void, pdwHandleCount: *mut u32) -> i32;
        pub fn K32GetProcessMemoryInfo(
            hProcess: *mut c_void,
            ppsmemCounters: *mut PROCESS_MEMORY_COUNTERS,
            cb: u32,
        ) -> i32;
        pub fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut c_void;
        pub fn Thread32First(hSnapshot: *mut c_void, lpte: *mut THREADENTRY32) -> i32;
        pub fn Thread32Next(hSnapshot: *mut c_void, lpte: *mut THREADENTRY32) -> i32;
        pub fn CloseHandle(hObject: *mut c_void) -> i32;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub level: String,      // INFO, DEBUG, WARN, ERROR, TRACE
    pub category: String,   // IPC, SWF, BACKUP, LAUNCHER, SYS, MEMORY, RENDER
    pub message: String,
    pub details: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevMetrics {
    pub pid: u32,
    pub cpu_percent: f64,
    pub memory_working_set_mb: f64,
    pub memory_peak_mb: f64,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub total_logs_recorded: u64,
    pub active_log_file: Option<String>,
    pub dev_mode_enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct DevConfigPersisted {
    pub enabled: bool,
    pub log_file: Option<String>,
}

pub struct DevTelemetry {
    pub enabled: AtomicBool,
    pub log_file_path: Mutex<Option<PathBuf>>,
    pub ring_buffer: Mutex<VecDeque<DevLogEntry>>,
    pub seq: AtomicU64,
    pub start_time: Instant,
    pub last_cpu_sample: Mutex<(Instant, u64)>, // (instant, total_process_time_100ns)
}

static TELEMETRY: std::sync::OnceLock<DevTelemetry> = std::sync::OnceLock::new();

fn get_telemetry() -> &'static DevTelemetry {
    TELEMETRY.get_or_init(|| {
        let config = load_persisted_config();
        DevTelemetry {
            enabled: AtomicBool::new(config.enabled),
            log_file_path: Mutex::new(config.log_file.map(PathBuf::from)),
            ring_buffer: Mutex::new(VecDeque::with_capacity(1000)),
            seq: AtomicU64::new(1),
            start_time: Instant::now(),
            last_cpu_sample: Mutex::new((Instant::now(), 0)),
        }
    })
}

fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| {
            let primary = d.join(".skinner_dev_config.json");
            if primary.exists() {
                primary
            } else {
                let legacy = d.join(".brawlcraft_dev_config.json");
                if legacy.exists() { legacy } else { primary }
            }
        }))
        .unwrap_or_else(|| PathBuf::from(".skinner_dev_config.json"))
}

fn load_persisted_config() -> DevConfigPersisted {
    DevConfigPersisted {
        enabled: false,
        log_file: None,
    }
}

fn save_persisted_config(_enabled: bool, _log_file: Option<String>) {
    let p = config_path();
    if p.is_file() {
        let _ = std::fs::remove_file(p);
    }
}

pub fn is_enabled() -> bool {
    get_telemetry().enabled.load(Ordering::Relaxed)
}

pub fn set_enabled(enabled: bool) {
    let t = get_telemetry();
    t.enabled.store(enabled, Ordering::Relaxed);
    let path = t.log_file_path.lock().unwrap().as_ref().map(|p| p.to_string_lossy().to_string());
    save_persisted_config(enabled, path);

    log(
        "INFO",
        "SYS",
        &format!("Developer Mode {}", if enabled { "ENABLED" } else { "DISABLED" }),
        None,
    );
}

pub fn set_log_file_path(path: Option<PathBuf>) {
    let t = get_telemetry();
    let path_str = path.as_ref().map(|p| p.to_string_lossy().to_string());
    *t.log_file_path.lock().unwrap() = path;
    save_persisted_config(t.enabled.load(Ordering::Relaxed), path_str);

    log(
        "INFO",
        "SYS",
        &format!(
            "Developer log file target updated: {:?}",
            t.log_file_path.lock().unwrap().as_ref().map(|p| p.display().to_string())
        ),
        None,
    );
}

pub fn get_log_file_path() -> Option<String> {
    get_telemetry()
        .log_file_path
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn log(level: &str, category: &str, message: &str, details: Option<&str>) {
    let t = get_telemetry();
    let enabled = t.enabled.load(Ordering::Relaxed);

    // If developer mode is disabled and no log file is set, we skip TRACE and DEBUG
    // but allow INFO/WARN/ERROR so recent logs are available if enabled.
    if !enabled && (level == "DEBUG" || level == "TRACE") {
        return;
    }

    let id = t.seq.fetch_add(1, Ordering::Relaxed);
    let now = chrono::Local::now();
    let timestamp = now.format("%H:%M:%S%.3f").to_string();
    let date_str = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    let entry = DevLogEntry {
        id,
        timestamp,
        level: level.to_string(),
        category: category.to_string(),
        message: message.to_string(),
        details: details.map(|s| s.to_string()),
    };

    // Store in ring buffer (max 1000 items)
    {
        let mut buffer = t.ring_buffer.lock().unwrap();
        if buffer.len() >= 1000 {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }

    // Write to persistent developer log file if configured
    let file_opt = t.log_file_path.lock().unwrap().clone();
    if let Some(path) = file_opt {
        let details_line = match details {
            Some(d) => format!("\n    Details: {}", d),
            None => String::new(),
        };
        let line = format!(
            "[{}] [{:5}] [{:8}] {}{}\n",
            date_str, level, category, message, details_line
        );

        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

pub fn get_recent_logs(since_id: u64, limit: usize) -> Vec<DevLogEntry> {
    let t = get_telemetry();
    let buffer = t.ring_buffer.lock().unwrap();
    buffer
        .iter()
        .filter(|e| e.id > since_id)
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

pub fn clear_logs() {
    let t = get_telemetry();
    let mut buffer = t.ring_buffer.lock().unwrap();
    buffer.clear();
}

pub fn get_metrics() -> DevMetrics {
    let t = get_telemetry();
    let uptime_sec = t.start_time.elapsed().as_secs();
    let hours = uptime_sec / 3600;
    let mins = (uptime_sec % 3600) / 60;
    let secs = uptime_sec % 60;
    let uptime_formatted = format!("{:02}:{:02}:{:02}", hours, mins, secs);

    #[cfg(windows)]
    {
        use win_metrics::*;
        unsafe {
            let pid = GetCurrentProcessId();
            let process_handle = GetCurrentProcess();

            // 1. Memory
            let mut mem = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                PageFaultCount: 0,
                PeakWorkingSetSize: 0,
                WorkingSetSize: 0,
                QuotaPeakPagedPoolUsage: 0,
                QuotaPagedPoolUsage: 0,
                QuotaPeakNonPagedPoolUsage: 0,
                QuotaNonPagedPoolUsage: 0,
                PagefileUsage: 0,
                PeakPagefileUsage: 0,
            };
            let _ = K32GetProcessMemoryInfo(
                process_handle,
                &mut mem,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            );

            let working_set_mb = (mem.WorkingSetSize as f64) / (1024.0 * 1024.0);
            let peak_mb = (mem.PeakWorkingSetSize as f64) / (1024.0 * 1024.0);

            // 2. Handles
            let mut handle_count = 0u32;
            let _ = GetProcessHandleCount(process_handle, &mut handle_count);

            // 3. Threads
            let mut thread_count = 1u32;
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot != INVALID_HANDLE_VALUE {
                let mut te: THREADENTRY32 = std::mem::zeroed();
                te.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
                if Thread32First(snapshot, &mut te) != 0 {
                    let mut count = 0;
                    loop {
                        if te.th32OwnerProcessID == pid {
                            count += 1;
                        }
                        if Thread32Next(snapshot, &mut te) == 0 {
                            break;
                        }
                    }
                    if count > 0 {
                        thread_count = count;
                    }
                }
                CloseHandle(snapshot);
            }

            // 4. CPU Calculation
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            let mut cpu_percent = 0.0;

            if GetProcessTimes(
                process_handle,
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            ) != 0
            {
                let total_cpu_100ns = kernel.to_u64() + user.to_u64();
                let now = Instant::now();
                let mut last = t.last_cpu_sample.lock().unwrap();
                let elapsed_duration = now.duration_since(last.0);
                let elapsed_100ns = elapsed_duration.as_nanos() as u64 / 100;

                if elapsed_100ns > 0 && last.1 > 0 && total_cpu_100ns >= last.1 {
                    let cpus = std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(1) as f64;
                    let process_diff = (total_cpu_100ns - last.1) as f64;
                    cpu_percent = ((process_diff / elapsed_100ns as f64) / cpus) * 100.0;
                    cpu_percent = cpu_percent.clamp(0.0, 100.0);
                }

                *last = (now, total_cpu_100ns);
            }

            DevMetrics {
                pid,
                cpu_percent: (cpu_percent * 10.0).round() / 10.0,
                memory_working_set_mb: (working_set_mb * 10.0).round() / 10.0,
                memory_peak_mb: (peak_mb * 10.0).round() / 10.0,
                thread_count,
                handle_count,
                uptime_seconds: uptime_sec,
                uptime_formatted,
                total_logs_recorded: t.seq.load(Ordering::Relaxed).saturating_sub(1),
                active_log_file: t
                    .log_file_path
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                dev_mode_enabled: t.enabled.load(Ordering::Relaxed),
            }
        }
    }

    #[cfg(not(windows))]
    {
        DevMetrics {
            pid: std::process::id(),
            cpu_percent: 0.0,
            memory_working_set_mb: 0.0,
            memory_peak_mb: 0.0,
            thread_count: 1,
            handle_count: 0,
            uptime_seconds: uptime_sec,
            uptime_formatted,
            total_logs_recorded: t.seq.load(Ordering::Relaxed).saturating_sub(1),
            active_log_file: t
                .log_file_path
                .lock()
                .unwrap()
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            dev_mode_enabled: t.enabled.load(Ordering::Relaxed),
        }
    }
}

pub fn export_diagnostics_report(
    target_path: &Path,
    system_info: &serde_json::Value,
) -> Result<(), String> {
    let metrics = get_metrics();
    let logs = get_recent_logs(0, 1000);

    let mut report = String::new();
    report.push_str("================================================================================\n");
    report.push_str("                      SKINNER DEVELOPER DIAGNOSTICS REPORT                      \n");
    report.push_str("================================================================================\n\n");
    report.push_str(&format!("Generated At:       {}\n", chrono::Local::now().to_rfc3339()));
    report.push_str(&format!("Process ID (PID):   {}\n", metrics.pid));
    report.push_str(&format!("Process Uptime:     {} ({} seconds)\n", metrics.uptime_formatted, metrics.uptime_seconds));
    report.push_str(&format!("CPU Usage:          {:.1}%\n", metrics.cpu_percent));
    report.push_str(&format!("RAM Working Set:    {:.1} MB (Peak: {:.1} MB)\n", metrics.memory_working_set_mb, metrics.memory_peak_mb));
    report.push_str(&format!("Active Threads:     {}\n", metrics.thread_count));
    report.push_str(&format!("Kernel Handles:     {}\n", metrics.handle_count));
    report.push_str(&format!("Total Logs Stored:  {}\n\n", metrics.total_logs_recorded));

    report.push_str("--------------------------------------------------------------------------------\n");
    report.push_str("SYSTEM & GAME INFORMATION\n");
    report.push_str("--------------------------------------------------------------------------------\n");
    report.push_str(&serde_json::to_string_pretty(system_info).unwrap_or_default());
    report.push_str("\n\n");

    report.push_str("--------------------------------------------------------------------------------\n");
    report.push_str("REAL-TIME OPERATION LOGS (CHRONOLOGICAL)\n");
    report.push_str("--------------------------------------------------------------------------------\n");
    for entry in logs {
        let details = match entry.details {
            Some(d) => format!("\n    Metadata: {}", d),
            None => String::new(),
        };
        report.push_str(&format!(
            "[{}] [{:5}] [{:8}] #{:04} - {}{}\n",
            entry.timestamp, entry.level, entry.category, entry.id, entry.message, details
        ));
    }
    report.push_str("\n================================================================================\n");
    report.push_str("                            END OF DIAGNOSTICS DUMP                             \n");
    report.push_str("================================================================================\n");

    if let Some(parent) = target_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut file = File::create(target_path)
        .map_err(|e| format!("Failed to create report file at {}: {}", target_path.display(), e))?;
    file.write_all(report.as_bytes())
        .map_err(|e| format!("Failed to write report content: {}", e))?;

    Ok(())
}
