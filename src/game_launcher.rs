#![allow(dead_code)]
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
mod win_proc {
    use std::ffi::c_void;

    pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    pub const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    pub const PROCESS_TERMINATE: u32 = 0x0001;

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct PROCESSENTRY32W {
        pub dwSize: u32,
        pub cntUsage: u32,
        pub th32ProcessID: u32,
        pub th32DefaultHeapID: usize,
        pub th32ModuleID: u32,
        pub cntThreads: u32,
        pub th32ParentProcessID: u32,
        pub pcPriClassBase: i32,
        pub dwFlags: u32,
        pub szExeFile: [u16; 260],
    }

    extern "system" {
        pub fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut c_void;
        pub fn Process32FirstW(hSnapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn Process32NextW(hSnapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut c_void;
        pub fn TerminateProcess(hProcess: *mut c_void, uExitCode: u32) -> i32;
        pub fn CloseHandle(hObject: *mut c_void) -> i32;
    }
}

pub struct GameLauncher;

impl GameLauncher {
    /// High-performance native process check.
    /// Uses Win32 Toolhelp32 snapshot directly with zero child process overhead.
    pub fn is_game_running() -> bool {
        #[cfg(windows)]
        {
            use win_proc::*;
            unsafe {
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                if snapshot != INVALID_HANDLE_VALUE {
                    let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

                    if Process32FirstW(snapshot, &mut entry) != 0 {
                        loop {
                            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                            let name_lower = name.to_ascii_lowercase();

                            if name_lower == "brawlhalla.exe" || name_lower == "brawlhallaair.exe" {
                                CloseHandle(snapshot);
                                return true;
                            }

                            if Process32NextW(snapshot, &mut entry) == 0 {
                                break;
                            }
                        }
                    }
                    CloseHandle(snapshot);
                    return false;
                }
            }
        }

        // Cross-platform or fallback if snapshot fails
        let mut cmd = Command::new("tasklist");
        cmd.args(["/FI", "IMAGENAME eq Brawlhalla*", "/FO", "CSV", "/NH"]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(out) = cmd.output() {
            let s = String::from_utf8_lossy(&out.stdout);
            let s_lower = s.to_lowercase();
            return s_lower.contains("brawlhalla.exe") || s_lower.contains("brawlhallaair.exe");
        }
        false
    }

    pub fn bring_game_to_front() -> bool {
        #[cfg(windows)]
        {
            use std::ffi::c_void;
            extern "system" {
                fn FindWindowA(lpClassName: *const u8, lpWindowName: *const u8) -> *mut c_void;
                fn ShowWindow(hWnd: *mut c_void, nCmdShow: i32) -> i32;
                fn SetForegroundWindow(hWnd: *mut c_void) -> i32;
            }

            unsafe {
                for title in [b"Brawlhalla\0".as_ptr(), b"Brawlhalla Air\0".as_ptr()] {
                    let hwnd = FindWindowA(std::ptr::null(), title);
                    if !hwnd.is_null() {
                        ShowWindow(hwnd, 9); // SW_RESTORE
                        SetForegroundWindow(hwnd);
                        return true;
                    }
                }
            }
        }
        true
    }

    pub fn launch(exe_path: &str, prefer_steam: bool) -> bool {
        if prefer_steam {
            let res = open::that("steam://rungameid/291550");
            if res.is_ok() {
                return true;
            }
        }

        if std::path::Path::new(exe_path).is_file() {
            let parent = std::path::Path::new(exe_path).parent();
            let mut cmd = Command::new(exe_path);
            if let Some(p) = parent {
                cmd.current_dir(p);
            }
            // Do NOT pass CREATE_NO_WINDOW for game GUI executable!
            return cmd.spawn().is_ok();
        }

        false
    }

    /// Fast and safe process termination.
    /// Terminates target game instances directly with native Win32 APIs,
    /// followed by clean handle release and zero process-loop overhead.
    pub fn kill_game_processes() {
        #[cfg(windows)]
        {
            use win_proc::*;
            let mut pids_to_kill = Vec::new();

            unsafe {
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                if snapshot != INVALID_HANDLE_VALUE {
                    let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

                    if Process32FirstW(snapshot, &mut entry) != 0 {
                        loop {
                            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                            let name_lower = name.to_ascii_lowercase();

                            if name_lower.starts_with("brawlhalla") && name_lower.ends_with(".exe") {
                                pids_to_kill.push(entry.th32ProcessID);
                            }

                            if Process32NextW(snapshot, &mut entry) == 0 {
                                break;
                            }
                        }
                    }
                    CloseHandle(snapshot);
                }

                // 1. Terminate directly via Win32 TerminateProcess
                for &pid in &pids_to_kill {
                    let h_proc = OpenProcess(PROCESS_TERMINATE, 0, pid);
                    if !h_proc.is_null() {
                        let _ = TerminateProcess(h_proc, 1);
                        CloseHandle(h_proc);
                    }
                }
            }

            // 2. Fallback to taskkill if EAC or permissions restricted direct handles
            if Self::is_game_running() {
                let mut k1 = Command::new("taskkill");
                k1.args(["/F", "/T", "/IM", "Brawlhalla.exe"]);
                k1.creation_flags(CREATE_NO_WINDOW);
                let _ = k1.output();

                let mut k2 = Command::new("taskkill");
                k2.args(["/F", "/T", "/IM", "BrawlhallaAir.exe"]);
                k2.creation_flags(CREATE_NO_WINDOW);
                let _ = k2.output();
            }
        }

        // Wait up to 1.5 seconds for processes to fully exit and release file handles
        for _ in 0..15 {
            if !Self::is_game_running() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    pub fn restart_game(exe_path: &str) -> bool {
        Self::kill_game_processes();
        Self::launch(exe_path, true)
    }
}
