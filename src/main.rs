#![windows_subsystem = "windows"]

#[cfg(windows)]
fn ensure_webview2_loader() {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let target = parent.join("WebView2Loader.dll");
            if !target.exists() {
                let bytes = include_bytes!("../assets/WebView2Loader.dll");
                let _ = std::fs::write(&target, bytes);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    brawlcraft_slint::desktop_app::log_startup(
        1,
        "Application process started",
        Some(&format!("PID: {}, Version: 3.3.4", std::process::id())),
    );

    std::panic::set_hook(Box::new(|info| {
        let err_text = format!("PANIC: {:?}", info);
        let _ = std::fs::write("panic_err.txt", &err_text);
        brawlcraft_slint::desktop_app::log_startup(9, "Fatal application panic", Some(&err_text));
    }));

    #[cfg(windows)]
    ensure_webview2_loader();

    if let Err(e) = brawlcraft_slint::desktop_app::run_desktop_app() {
        let err_text = format!("Error: {:?}", e);
        let _ = std::fs::write("panic_err.txt", &err_text);
        brawlcraft_slint::desktop_app::log_startup(4, "Desktop application run error", Some(&err_text));
    }
    Ok(())
}
