fn main() {
    println!("cargo:rerun-if-changed=frontend_dist");

    #[cfg(windows)]
    {
        use std::path::{Path, PathBuf};
        use std::process::Command;

        if let Ok(out_dir_str) = std::env::var("OUT_DIR") {
            let out_dir = PathBuf::from(&out_dir_str);
            let rc_path = out_dir.join("resource.rc");
            let obj_path = out_dir.join("resource.o");
            let lib_path = out_dir.join("libresource.a");

            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
            let icon_abs = Path::new(&manifest_dir).join("assets").join("app.ico");
            let icon_str = icon_abs.to_string_lossy().replace('\\', "/");

            let pkg_ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "3.3.3".into());
            let v_parts: Vec<&str> = pkg_ver.split('.').collect();
            let v_commas = format!(
                "{},{},{},0",
                v_parts.first().unwrap_or(&"3"),
                v_parts.get(1).unwrap_or(&"3"),
                v_parts.get(2).unwrap_or(&"3")
            );

            let rc_content = format!(
                r#"1 ICON "{}"
1 VERSIONINFO
FILEVERSION {}
PRODUCTVERSION {}
FILEOS 0x40004
FILETYPE 0x1
{{
    BLOCK "StringFileInfo"
    {{
        BLOCK "040904b0"
        {{
            VALUE "CompanyName", "Skinner"
            VALUE "FileDescription", "Skinner - Ultimate Brawlhalla Skin Studio"
            VALUE "FileVersion", "{}"
            VALUE "InternalName", "Skinner"
            VALUE "OriginalFilename", "Skinner.exe"
            VALUE "ProductName", "Skinner"
            VALUE "ProductVersion", "{}"
        }}
    }}
    BLOCK "VarFileInfo"
    {{
        VALUE "Translation", 0x0409, 0x04b0
    }}
}}
"#,
                icon_str, v_commas, v_commas, pkg_ver, pkg_ver
            );

            if std::fs::write(&rc_path, rc_content).is_ok() {
                let windres_cmd = Command::new("C:/mingw64/bin/windres.exe")
                    .arg("-i")
                    .arg(&rc_path)
                    .arg("-o")
                    .arg(&obj_path)
                    .status();

                if let Ok(ws) = windres_cmd {
                    if ws.success() {
                        let ar_cmd = Command::new("C:/mingw64/bin/ar.exe")
                            .arg("rcs")
                            .arg(&lib_path)
                            .arg(&obj_path)
                            .status();

                        if let Ok(ars) = ar_cmd {
                            if ars.success() {
                                println!("cargo:rustc-link-search=native={}", out_dir.display());
                                println!("cargo:rustc-link-lib=static=resource");
                                println!("cargo:rustc-link-arg={}", obj_path.to_string_lossy().replace('\\', "/"));
                            }
                        }
                    }
                }
            }

            if let Some(target_dir) = out_dir.ancestors().nth(3) {
                let dll_src = std::path::Path::new("assets/WebView2Loader.dll");
                if dll_src.exists() {
                    let _ = std::fs::copy(dll_src, target_dir.join("WebView2Loader.dll"));
                }
            }
        }
    }
}
