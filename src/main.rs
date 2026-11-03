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
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("panic_err.txt", format!("PANIC: {:?}", info));
    }));

    #[cfg(windows)]
    ensure_webview2_loader();

    if let Err(e) = brawlcraft_slint::desktop_app::run_desktop_app() {
        let _ = std::fs::write("panic_err.txt", format!("Error: {:?}", e));
    }
    Ok(())
}
