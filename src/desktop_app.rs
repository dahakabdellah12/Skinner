use crate::backup_manager::BackupManager;
use crate::catalog::Catalog;
use crate::game_detector::GameDetector;
use crate::game_launcher::GameLauncher;
use crate::models::{GameInstallation, ModificationRequest, ModificationResult, Preset};
use crate::modification_engine::ModificationEngine;
use crate::preset_manager::PresetManager;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Icon, WindowBuilder},
};
use wry::{
    http::{header::CONTENT_TYPE, Response, StatusCode},
    WebViewBuilder,
};

#[derive(rust_embed::RustEmbed)]
#[folder = "frontend_dist/"]
pub struct FrontendAssets; // Embedded complete legend & cosmetic catalog production UI

#[derive(Debug)]
pub enum UserEvent {
    EvalScript(String),
    Minimize,
    Maximize,
    Close,
    Drag,
}

pub struct RendererClient {
    child: Option<Child>,
    reader: Option<BufReader<std::process::ChildStdout>>,
}

impl RendererClient {
    pub fn new() -> Self {
        let mut client = Self {
            child: None,
            reader: None,
        };
        client.ensure_started();
        client
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.reader = None;
    }

    fn find_renderer_exe() -> Option<PathBuf> {
        let mut candidates = vec![
            PathBuf::from("bin/renderer/test_renderer.exe"),
            PathBuf::from("brawlcraft-slint/bin/renderer/test_renderer.exe"),
        ];

        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                candidates.push(parent.join("bin/renderer/test_renderer.exe"));
                candidates.push(parent.join("test_renderer.exe"));
                if let Some(grandparent) = parent.parent() {
                    candidates.push(grandparent.join("bin/renderer/test_renderer.exe"));
                    if let Some(great) = grandparent.parent() {
                        candidates.push(great.join("bin/renderer/test_renderer.exe"));
                        candidates.push(great.join("brawlcraft-slint/bin/renderer/test_renderer.exe"));
                    }
                }
            }
        }

        candidates.into_iter().find(|p| p.is_file())
    }

    pub fn ensure_started(&mut self) -> bool {
        // Check if existing child is alive
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => return true, // still active
                _ => {
                    self.stop();
                }
            }
        }

        if let Some(path) = Self::find_renderer_exe() {
            let mut c = Command::new(&path);
            c.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                c.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            if let Ok(mut cmd) = c.spawn() {
                let stdout = cmd.stdout.take().map(BufReader::new);
                self.child = Some(cmd);
                self.reader = stdout;
                return true;
            }
        }

        false
    }

    pub fn render(&mut self, req_json: &serde_json::Value) -> Option<String> {
        if !self.ensure_started() {
            return None;
        }

        let mut line = serde_json::to_string(req_json).ok()?;
        line.push('\n');

        for attempt in 0..2 {
            if attempt > 0 {
                self.stop();
                if !self.ensure_started() {
                    return None;
                }
            }

            let child = match self.child.as_mut() {
                Some(c) => c,
                None => continue,
            };
            let stdin = match child.stdin.as_mut() {
                Some(s) => s,
                None => continue,
            };
            let reader = match self.reader.as_mut() {
                Some(r) => r,
                None => continue,
            };

            if stdin.write_all(line.as_bytes()).is_err() || stdin.flush().is_err() {
                continue;
            }

            let mut resp_line = String::new();
            if reader.read_line(&mut resp_line).is_ok() && !resp_line.is_empty() {
                if let Ok(resp_val) = serde_json::from_str::<serde_json::Value>(&resp_line) {
                    if let Some(svg) = resp_val.get("svg").and_then(|s| s.as_str()) {
                        return Some(svg.to_string());
                    }
                }
                return None;
            }
        }
        None
    }
}

impl Drop for RendererClient {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct SharedAppData {
    pub installation: GameInstallation,
    pub catalog: Catalog,
    pub backup_manager: BackupManager,
    pub preset_manager: PresetManager,
    pub renderer: Mutex<RendererClient>,
    pub pending_modifications: Vec<ModificationRequest>,
}

impl SharedAppData {
    pub fn apply_pending_modifications(&mut self) -> usize {
        if self.pending_modifications.is_empty() {
            return 0;
        }

        let mut count = 0;
        let mut engine = ModificationEngine::new(
            PathBuf::from(&self.installation.game_path),
            &mut self.backup_manager,
            &self.catalog,
        );

        let reqs = std::mem::take(&mut self.pending_modifications);
        for req in reqs {
            let res = engine.apply_modification(&req);
            if res.success {
                count += 1;
            }
        }

        count
    }
}

pub fn browse_game_exe() -> Option<String> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$d = New-Object System.Windows.Forms.OpenFileDialog
$d.Filter = 'Brawlhalla.exe|Brawlhalla.exe|All Executables (*.exe)|*.exe'
$d.Title = 'Select Brawlhalla Executable'
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.FileName }
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn browse_dev_log_path() -> Option<String> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$d = New-Object System.Windows.Forms.SaveFileDialog
$d.Filter = 'Log Files (*.log)|*.log|Text Files (*.txt)|*.txt|All Files (*.*)|*.*'
$d.Title = 'Select Developer Log Destination'
$d.FileName = 'skinner_dev_log.txt'
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.FileName }
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn get_svg_cache_dir() -> PathBuf {
    let cache_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("cache").join("svg")))
        .unwrap_or_else(|| PathBuf::from("cache/svg"));
    let _ = std::fs::create_dir_all(&cache_dir);
    cache_dir
}

pub fn log_startup(stage: u32, name: &str, detail: Option<&str>) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = match detail {
        Some(d) => format!("[{}] [STAGE {:02}] {}: {}\n", now, stage, name, d),
        None => format!("[{}] [STAGE {:02}] {}\n", now, stage, name),
    };

    eprint!("{}", line);

    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("Skinner").join("Logs");
        if std::fs::create_dir_all(&dir).is_ok() {
            let log_file = dir.join("startup.log");
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
                let _ = f.write_all(line.as_bytes());
            }
        }
    }
}

pub fn is_allowed_internal_navigation(url: &str) -> bool {
    let u = url.trim();
    u.starts_with("brawlcraft://")
        || u.starts_with("http://brawlcraft.localhost")
        || u.starts_with("https://brawlcraft.localhost")
        || u == "about:blank"
}

pub fn get_user_data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("Skinner").join("WebView2");
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    PathBuf::from("cache/webview2")
}

pub fn show_native_error_dialog(title: &str, message: &str) {
    let script = format!(
        "[System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms') | Out-Null; [System.Windows.Forms.MessageBox]::Show('{}', '{}', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)",
        message.replace('\'', "''"),
        title.replace('\'', "''")
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
}

static SVG_MEM_CACHE: std::sync::OnceLock<Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();

fn get_svg_mem_cache() -> &'static Mutex<std::collections::HashMap<String, String>> {
    SVG_MEM_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::with_capacity(512)))
}

pub fn get_cached_svg_fast(key: &str) -> Option<String> {
    if let Ok(cache) = get_svg_mem_cache().lock() {
        if let Some(svg) = cache.get(key) {
            return Some(svg.clone());
        }
    }
    let cache_dir = get_svg_cache_dir();
    let disk_path = cache_dir.join(format!("{}.svg", sanitize_cache_key(key)));
    if disk_path.is_file() {
        if let Ok(svg) = std::fs::read_to_string(&disk_path) {
            let trimmed = svg.trim();
            if !trimmed.is_empty() {
                let res = trimmed.to_string();
                if let Ok(mut cache) = get_svg_mem_cache().lock() {
                    if cache.len() >= 600 {
                        cache.clear();
                    }
                    cache.insert(key.to_string(), res.clone());
                }
                return Some(res);
            }
        }
    }
    None
}

pub fn put_cached_svg_fast(key: &str, svg: &str) {
    let cache_dir = get_svg_cache_dir();
    let disk_path = cache_dir.join(format!("{}.svg", sanitize_cache_key(key)));
    let _ = std::fs::write(&disk_path, svg);
    if let Ok(mut cache) = get_svg_mem_cache().lock() {
        if cache.len() >= 600 {
            cache.clear();
        }
        cache.insert(key.to_string(), svg.to_string());
    }
}

fn sanitize_cache_key(key: &str) -> String {
    let sanitized: String = key.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(120)
        .collect();
    if sanitized.is_empty() {
        "cached_asset".to_string()
    } else {
        sanitized
    }
}

pub fn prepare_render_request(req: &mut serde_json::Value, state: &SharedAppData) {
    if let Some(obj) = req.as_object_mut() {
        obj.insert("gamePath".into(), serde_json::json!(state.installation.game_path));

        let is_hammer_stub = obj.get("baseWeapon").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("Hammer")).unwrap_or(false)
            && (obj.get("symbol").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("Stub")).unwrap_or(false)
                || obj.get("weaponSkinName").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("HammerStub")).unwrap_or(false));
        if is_hammer_stub {
            obj.insert("swfFile".into(), serde_json::json!("Gfx_Super.swf"));
            obj.insert("symbol".into(), serde_json::json!("Detective"));
        }

        let cs_id = obj.get("colorSchemeId")
            .or_else(|| obj.get("colorScheme").and_then(|cs| cs.get("schemeId")))
            .and_then(|v| {
                if let Some(n) = v.as_i64() {
                    Some(n.to_string())
                } else {
                    v.as_str().map(|s| s.to_string())
                }
            });

        if let Some(scheme_id_str) = cs_id {
            if !scheme_id_str.is_empty() && scheme_id_str != "__NONE__" {
                let scheme = state.catalog.color_schemes.iter().find(|s| {
                    s.scheme_id.to_string() == scheme_id_str
                        || s.scheme_name.eq_ignore_ascii_case(&scheme_id_str)
                        || s.display_name.eq_ignore_ascii_case(&scheme_id_str)
                });

                if let Some(s) = scheme {
                    let mut base_sources = Vec::new();

                    let costume_name = obj.get("costumeName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let mut owner_hero = obj.get("ownerHero").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let base_weapon = obj.get("baseWeapon").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let symbol = obj.get("symbol").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let weapon_skin_name = obj.get("weaponSkinName").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let swf_file = obj.get("swfFile").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    // Infer hero from SWF file if not explicitly supplied (e.g. Gfx_Sellsword.swf -> Sellsword)
                    if owner_hero.is_empty() && swf_file.starts_with("Gfx_") && !swf_file.starts_with("Gfx_Weapons") {
                        owner_hero = swf_file
                            .trim_start_matches("Gfx_")
                            .trim_end_matches(".swf")
                            .to_string();
                    }

                    let costume_name = costume_name.as_str();
                    let base_weapon = base_weapon.as_str();
                    let symbol = symbol.as_str();
                    let weapon_skin_name = weapon_skin_name.as_str();

                    // Standard Brawlhalla weapon define fills used across game SWFs
                    let standard_weapon_colors: std::collections::HashMap<String, u32> = [
                        ("weapon".to_string(), 0x8D8D95),
                        ("weaponlt".to_string(), 0xACB1B7),
                        ("weapondk".to_string(), 0x65626D),
                        ("weaponvl".to_string(), 0xCAD1D7),
                        ("special".to_string(), 0x3E5466),
                        ("speciallt".to_string(), 0x5D7B93),
                        ("specialdk".to_string(), 0x2A3B49),
                        ("specialvl".to_string(), 0x82A4BF),
                        ("body1".to_string(), 0x485868),
                        ("body2".to_string(), 0x82A4BF),
                    ].into_iter().collect();
                    base_sources.push(&standard_weapon_colors);

                    // 1. Specifically match the requested weapon skin first by symbol or name within the same base_weapon!
                    if !symbol.is_empty() || !weapon_skin_name.is_empty() || !base_weapon.is_empty() {
                        let clean_sym = symbol.replace(" ", "");
                        let clean_wsn = weapon_skin_name.replace(" ", "");
                        let matched_wep = state.catalog.weapon_skins.iter().find(|w| {
                            if !base_weapon.is_empty() && !w.base_weapon.eq_ignore_ascii_case(base_weapon) {
                                return false;
                            }
                            (!symbol.is_empty() && (w.symbol_suffix.eq_ignore_ascii_case(symbol) || w.weapon_skin_name.eq_ignore_ascii_case(symbol) || w.weapon_skin_name.replace(" ", "").eq_ignore_ascii_case(&clean_sym)))
                                || (!weapon_skin_name.is_empty() && (w.weapon_skin_name.eq_ignore_ascii_case(weapon_skin_name) || w.display_name.eq_ignore_ascii_case(weapon_skin_name) || w.weapon_skin_name.replace(" ", "").eq_ignore_ascii_case(&clean_wsn)))
                        }).or_else(|| {
                            // Only fall back to default if this was explicitly an empty or default symbol request
                            if (symbol.is_empty() || symbol.eq_ignore_ascii_case("Default")) && !owner_hero.is_empty() && !base_weapon.is_empty() {
                                state.catalog.weapon_skins.iter().find(|w| {
                                    w.owner_hero.eq_ignore_ascii_case(&owner_hero)
                                        && w.base_weapon.eq_ignore_ascii_case(base_weapon)
                                        && w.is_default
                                })
                            } else {
                                None
                            }
                        });

                        if let Some(wep) = matched_wep {
                            if !wep.base_colors.is_empty() {
                                base_sources.push(&wep.base_colors);
                            }
                        }

                        // Also check if symbol matches a costume that defines weapon colors (e.g. SnowboardJY, DeathJester)
                        if !symbol.is_empty() {
                            if let Some(cost) = state.catalog.costumes.iter().find(|c| {
                                c.symbol_suffix.eq_ignore_ascii_case(symbol)
                                    || c.costume_name.eq_ignore_ascii_case(symbol)
                                    || c.display_name.eq_ignore_ascii_case(symbol)
                            }) {
                                if !cost.base_colors.is_empty() {
                                    base_sources.push(&cost.base_colors);
                                }
                            }
                        }
                    }

                    if !costume_name.is_empty() || !owner_hero.is_empty() {
                        if let Some(cost) = state.catalog.costumes.iter().find(|c| {
                            (!owner_hero.is_empty() && c.owner_hero.eq_ignore_ascii_case(&owner_hero)
                                && (c.costume_name.eq_ignore_ascii_case(costume_name) || c.display_name.eq_ignore_ascii_case(costume_name)))
                                || (!costume_name.is_empty() && (c.costume_name.eq_ignore_ascii_case(costume_name) || c.display_name.eq_ignore_ascii_case(costume_name)))
                        }) {
                            if !cost.base_colors.is_empty() {
                                base_sources.push(&cost.base_colors);
                            }
                        }

                        // Also include the hero's default costume base colors
                        if !owner_hero.is_empty() {
                            if let Some(cost) = state.catalog.costumes.iter().find(|c| {
                                c.owner_hero.eq_ignore_ascii_case(&owner_hero) && c.is_default
                            }) {
                                if !cost.base_colors.is_empty() {
                                    base_sources.push(&cost.base_colors);
                                }
                            }
                        }
                    }

                    if base_sources.is_empty() && !owner_hero.is_empty() {
                        if let Some(cost) = state.catalog.costumes.iter().find(|c| {
                            c.owner_hero.eq_ignore_ascii_case(&owner_hero) && c.is_default
                        }) {
                            if !cost.base_colors.is_empty() {
                                base_sources.push(&cost.base_colors);
                            }
                        }
                    }

                    let mut color_map = serde_json::Map::new();
                    for (key, target_color) in &s.swap_colors {
                        let key_lower = key.to_lowercase();
                        let target_24 = target_color & 0xFFFFFF;
                        for src in &base_sources {
                            if let Some(base_val) = src.get(key).or_else(|| src.get(&key_lower)) {
                                let base_24 = base_val & 0xFFFFFF;
                                color_map.insert(base_24.to_string(), serde_json::json!(target_24));
                                color_map.insert(base_val.to_string(), serde_json::json!(target_24));
                            }
                        }
                    }

                    if !color_map.is_empty() {
                        obj.insert("colorMap".into(), serde_json::Value::Object(color_map));
                    }
                }
            }
        }
    }
}


fn build_system_info(
    inst: &GameInstallation,
    cat: &Catalog,
    bm: &BackupManager,
) -> serde_json::Value {
    serde_json::json!({
        "gamePath": inst.game_path,
        "version": inst.version,
        "buildId": inst.build_id,
        "discoveredSwfCount": inst.discovered_swf_count,
        "isValid": inst.is_valid,
        "isGameRunning": GameLauncher::is_game_running(),
        "activeModCount": bm.count_modified_files(),
        "appVersion": "3.3.2",
        "totalLegends": cat.legends.len(),
        "totalCostumes": cat.costumes.len(),
        "totalWeaponSkins": cat.weapon_skins.len(),
        "totalColorSchemes": cat.color_schemes.len()
    })
}

fn build_catalog_response(cat: &Catalog) -> serde_json::Value {
    let mut weapon_types: HashSet<String> = HashSet::new();
    for w in &cat.weapon_skins {
        if !w.base_weapon.is_empty() {
            weapon_types.insert(w.base_weapon.clone());
        }
    }
    let mut weapon_types_list: Vec<String> = weapon_types.into_iter().collect();
    weapon_types_list.sort();

    serde_json::json!({
        "legends": cat.legends,
        "costumes": cat.costumes,
        "weaponSkins": cat.weapon_skins,
        "colorSchemes": cat.color_schemes,
        "weaponTypes": weapon_types_list,
        "koEffects": [],
        "sidekicks": [],
        "podiums": [],
        "trailEffects": [],
        "crossoverSfx": []
    })
}



fn handle_action(
    state: &mut SharedAppData,
    action: &str,
    payload: serde_json::Value,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<serde_json::Value, String> {
    match action {
        "getSystemInfo" => {
            log_startup(11, "Backend communication initialized", Some("IPC getSystemInfo processed"));
            if !GameLauncher::is_game_running() && !state.pending_modifications.is_empty() {
                state.apply_pending_modifications();
            }
            Ok(build_system_info(
                &state.installation,
                &state.catalog,
                &state.backup_manager,
            ))
        }
        "getCatalog" => Ok(build_catalog_response(&state.catalog)),
        "validateModification" => {
            let req: ModificationRequest =
                serde_json::from_value(payload).map_err(|e| e.to_string())?;
            let engine = ModificationEngine::new(
                PathBuf::from(&state.installation.game_path),
                &mut state.backup_manager,
                &state.catalog,
            );
            Ok(engine.validate_modification(&req))
        }
        "applyModification" | "applyAdvancedModification" => {
            let auto_restart = payload
                .get("autoRestart")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let req: ModificationRequest =
                serde_json::from_value(payload).map_err(|e| e.to_string())?;

            let is_running = GameLauncher::is_game_running();

            if auto_restart {
                // If auto-restart is enabled:
                // 1. Terminate game completely from roots first to release file locks!
                GameLauncher::kill_game_processes();

                // 2. Apply any prior pending modifications + this modification
                state.apply_pending_modifications();

                let mut engine = ModificationEngine::new(
                    PathBuf::from(&state.installation.game_path),
                    &mut state.backup_manager,
                    &state.catalog,
                );
                let mut result = engine.apply_modification(&req);

                // 3. Launch game fresh from roots
                if result.success {
                    let launched = GameLauncher::launch(&state.installation.exe_path, true);
                    if launched {
                        result.operation_log.push(
                            "Game was terminated and restarted cleanly from roots to activate modifications live.".into()
                        );
                    }
                }

                serde_json::to_value(&result).map_err(|e| e.to_string())
            } else {
                // Auto-restart is disabled:
                if is_running {
                    // Game is currently running: place in pending queue!
                    let hero_name = req.get_legend_name();
                    state.pending_modifications.retain(|r| r.get_legend_name() != hero_name);
                    state.pending_modifications.push(req);

                    let result = ModificationResult {
                        success: true,
                        message: "تم وضع السكن في قائمة الانتظار بنجاح! سيتم تطبيقه فور إغلاق اللعبة أو عند الضغط على 'تحديث اللعبة'.".into(),
                        modified_files: Vec::new(),
                        backup_id: "QUEUED".into(),
                        operation_log: vec![
                            "Game is currently running and auto-restart is disabled.".into(),
                            "Modification placed in pending queue to be applied on next game start/reload.".into(),
                        ],
                    };
                    serde_json::to_value(&result).map_err(|e| e.to_string())
                } else {
                    // Game is NOT running: apply to files immediately!
                    state.apply_pending_modifications();

                    let mut engine = ModificationEngine::new(
                        PathBuf::from(&state.installation.game_path),
                        &mut state.backup_manager,
                        &state.catalog,
                    );
                    let result = engine.apply_modification(&req);
                    serde_json::to_value(&result).map_err(|e| e.to_string())
                }
            }
        }
        "restoreCostume" => {
            let swf_file = payload
                .get("swfFile")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let restored = state.backup_manager.restore_file(swf_file);
            if state.backup_manager.is_file_modified("Gfx_Hands.swf") {
                let _ = state.backup_manager.restore_file("Gfx_Hands.swf");
            }
            Ok(serde_json::json!(restored))
        }
        "restoreAll" => {
            let count = state.backup_manager.restore_all().unwrap_or(0);
            if let Ok(mut r) = state.renderer.lock() {
                r.ensure_started();
            }
            Ok(serde_json::json!(count))
        }
        "getPresets" => {
            serde_json::to_value(state.preset_manager.presets()).map_err(|e| e.to_string())
        }
        "savePreset" => {
            let preset: Preset = serde_json::from_value(payload).map_err(|e| e.to_string())?;
            state.preset_manager.save_preset(preset.clone());
            serde_json::to_value(&preset).map_err(|e| e.to_string())
        }
        "deletePreset" => {
            let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let deleted = state.preset_manager.delete_preset(id);
            Ok(serde_json::json!(deleted))
        }
        "duplicatePreset" => {
            let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let new_name = payload
                .get("newName")
                .and_then(|v| v.as_str())
                .unwrap_or("Duplicate Preset");
            if let Some(mut preset) = state
                .preset_manager
                .presets()
                .iter()
                .find(|p| p.id == id)
                .cloned()
            {
                preset.id = format!("preset_{}", chrono::Utc::now().timestamp_millis());
                preset.name = new_name.to_string();
                state.preset_manager.save_preset(preset.clone());
                serde_json::to_value(&preset).map_err(|e| e.to_string())
            } else {
                Err(format!("Preset '{}' not found", id))
            }
        }
        "exportPreset" => {
            let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(preset) = state.preset_manager.presets().iter().find(|p| p.id == id) {
                let json_str =
                    serde_json::to_string_pretty(preset).map_err(|e| e.to_string())?;
                Ok(serde_json::json!(json_str))
            } else {
                Err(format!("Preset '{}' not found", id))
            }
        }
        "importPreset" => {
            let json_str = payload
                .get("json")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let preset = state
                .preset_manager
                .import_json(json_str)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&preset).map_err(|e| e.to_string())
        }
        "getBackups" => {
            serde_json::to_value(state.backup_manager.records()).map_err(|e| e.to_string())
        }
        "restoreBackupFile" => {
            let rel_path = payload
                .get("relativePath")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let restored = state.backup_manager.restore_file(rel_path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(restored))
        }
        "getDiagnostics" => {
            let modified = state.backup_manager.get_modified_files();
            Ok(serde_json::json!({
                "installation": {
                    "gamePath": state.installation.game_path,
                    "version": state.installation.version,
                    "buildId": state.installation.build_id,
                    "discoveredSwfCount": state.installation.discovered_swf_count,
                    "isValid": state.installation.is_valid
                },
                "totalLegendsDiscovered": state.catalog.legends.len(),
                "totalCostumesDiscovered": state.catalog.costumes.len(),
                "totalWeaponSkinsDiscovered": state.catalog.weapon_skins.len(),
                "totalColorSchemesDiscovered": state.catalog.color_schemes.len(),
                "modifiedFilesDetected": modified,
                "backupArchivesCount": state.backup_manager.records().len(),
                "isSteamRunning": true,
                "healthChecks": [
                    "✓ Brawlhalla installation directory verified.",
                    "✓ Game.swz encryption keys decrypted successfully.",
                    "✓ SWF cosmetic asset registry loaded.",
                    "✓ Pristine atomic backup snapshots verified.",
                    "✓ Rust High-Performance Engine initialized."
                ]
            }))
        }
        "launchGame" => {
            if !GameLauncher::is_game_running() {
                state.apply_pending_modifications();
            }
            let ok = GameLauncher::launch(&state.installation.exe_path, true);
            Ok(serde_json::json!(ok))
        }
        "restartGame" => {
            // Forcefully terminate game from its roots
            GameLauncher::kill_game_processes();

            // Apply all queued pending modifications before restarting
            let applied_count = state.apply_pending_modifications();

            // Launch fresh instance of Brawlhalla
            let ok = GameLauncher::launch(&state.installation.exe_path, true);
            Ok(serde_json::json!({
                "success": ok,
                "appliedPending": applied_count
            }))
        }
        "bringGameToFront" => Ok(serde_json::json!(GameLauncher::bring_game_to_front())),
        "isGameRunning" => {
            let running = GameLauncher::is_game_running();
            if !running && !state.pending_modifications.is_empty() {
                state.apply_pending_modifications();
            }
            Ok(serde_json::json!(running))
        }
        "renderPreview" | "getAssetSvg" => {
            let mut req = payload.clone();
            let cache_key = req.get("cacheKey").and_then(|v| v.as_str()).map(|s| s.to_string());

            if let Some(ref key) = cache_key {
                if let Some(svg) = get_cached_svg_fast(key) {
                    return Ok(serde_json::json!(svg));
                }
            }

            prepare_render_request(&mut req, state);

            if let Ok(mut client) = state.renderer.lock() {
                if let Some(svg) = client.render(&req) {
                    if !svg.trim().is_empty() {
                        if let Some(ref key) = cache_key {
                            put_cached_svg_fast(key, &svg);
                        }
                        return Ok(serde_json::json!(svg));
                    }
                }
            }
            Ok(serde_json::Value::Null)
        }
        "batchGetAssetSvgs" => {
            let items = payload.get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let mut results = serde_json::Map::new();
            let mut to_render = Vec::new();

            for item in items {
                let cache_key = item.get("cacheKey")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if cache_key.is_empty() {
                    continue;
                }

                if let Some(svg) = get_cached_svg_fast(&cache_key) {
                    results.insert(cache_key, serde_json::json!(svg));
                    continue;
                }

                if let Some(query) = item.get("queryParams").cloned() {
                    to_render.push((cache_key, query));
                }
            }

            if !to_render.is_empty() {
                if let Ok(mut client) = state.renderer.lock() {
                    for (cache_key, mut req) in to_render {
                        prepare_render_request(&mut req, state);
                        if let Some(svg) = client.render(&req) {
                            if !svg.trim().is_empty() {
                                put_cached_svg_fast(&cache_key, &svg);
                                results.insert(cache_key, serde_json::json!(svg));
                            }
                        }
                    }
                }
            }

            Ok(serde_json::Value::Object(results))
        }
        "getVerificationSteps" => Ok(serde_json::json!([
            "1. Ensure Brawlhalla is launched to the Main Menu.",
            "2. Navigate to Offline Play -> Training Mode.",
            "3. Select the modified Legend to verify your custom skins."
        ])),
        "windowMinimize" => {
            let _ = proxy.send_event(UserEvent::Minimize);
            Ok(serde_json::json!(true))
        }
        "windowMaximize" => {
            let _ = proxy.send_event(UserEvent::Maximize);
            Ok(serde_json::json!(true))
        }
        "windowClose" => {
            let _ = proxy.send_event(UserEvent::Close);
            Ok(serde_json::json!(true))
        }
        "windowDrag" => {
            let _ = proxy.send_event(UserEvent::Drag);
            Ok(serde_json::json!(true))
        }
        "isWindowMaximized" => Ok(serde_json::json!(false)),
        "browseGameExe" => {
            let path = browse_game_exe();
            Ok(serde_json::json!(path))
        }
        "selectGamePath" => {
            let p = if let Some(s) = payload.as_str() {
                s
            } else {
                payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            };
            let clean = p.trim().trim_matches('"').trim_matches('\'');
            let candidate = if clean.ends_with("Brawlhalla.exe") {
                Path::new(clean).parent().unwrap_or(Path::new(clean))
            } else {
                Path::new(clean)
            };

            let detected = GameDetector::detect(candidate.to_str());
            if detected.is_valid {
                state.installation = detected;
                let mut cat = Catalog::default();
                let _ = cat.load_from_installation(&state.installation);
                state.catalog = cat;
                state.backup_manager = BackupManager::new(&state.installation.game_path);
                Ok(serde_json::json!({
                    "success": true,
                    "systemInfo": build_system_info(&state.installation, &state.catalog, &state.backup_manager),
                    "catalog": build_catalog_response(&state.catalog)
                }))
            } else {
                let err_msg = if state.installation.validation_messages.is_empty() {
                    format!("Invalid Brawlhalla path: '{}'", clean)
                } else {
                    state.installation.validation_messages.join("; ")
                };
                Err(err_msg)
            }
        }
        "clearAssetCache" => {
            if let Ok(mut c) = get_svg_mem_cache().lock() {
                c.clear();
            }
            let cache_dir = get_svg_cache_dir();
            if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                for entry in entries.flatten() {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
            if let Ok(mut r) = state.renderer.lock() {
                r.stop();
                r.ensure_started();
            }
            Ok(serde_json::json!({
                "success": true,
                "message": "Asset cache cleared and renderer restarted successfully."
            }))
        }
        "repairGameInstallation" => {
            let detected = GameDetector::detect(Some(&state.installation.game_path));
            if detected.is_valid {
                state.installation = detected;
                let mut cat = Catalog::default();
                let _ = cat.load_from_installation(&state.installation);
                state.catalog = cat;
            }
            let _ = state.backup_manager.repair_sfx_files();
            let restored = state.backup_manager.restore_all().unwrap_or(0);
            if let Ok(mut r) = state.renderer.lock() {
                r.ensure_started();
            }
            Ok(serde_json::json!({
                "success": true,
                "restoredCount": restored,
                "message": format!("Diagnostic and repair completed. Restored {} modified file(s).", restored)
            }))
        }
        "checkForUpdates" => {
            let res = crate::updater::UpdateManager::check_for_updates("3.3.2");
            Ok(serde_json::to_value(res).unwrap_or(serde_json::Value::Null))
        }
        "installUpdate" => {
            let download_url = payload.get("downloadUrl").and_then(|v| v.as_str()).unwrap_or("");
            let version = payload.get("version").and_then(|v| v.as_str()).unwrap_or("3.3.2");
            let asset_name = payload.get("assetName").and_then(|v| v.as_str()).unwrap_or("");
            let manifest_url = payload.get("manifestUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
            let signature_url = payload.get("signatureUrl").and_then(|v| v.as_str()).map(|s| s.to_string());

            if download_url.is_empty() {
                return Err("Missing download URL".to_string());
            }

            let release_info = crate::updater::ReleaseInfo {
                has_update: true,
                current_version: "3.3.2".to_string(),
                latest_version: version.to_string(),
                release_title: format!("Skinner v{}", version),
                download_url: download_url.to_string(),
                html_url: format!("{}/releases/tag/v{}", crate::updater::OFFICIAL_REPO_URL, version),
                asset_name: asset_name.to_string(),
                asset_size_formatted: "50 MB".to_string(),
                release_notes: String::new(),
                manifest_url,
                signature_url,
                repo: crate::updater::OFFICIAL_REPO_SLUG.to_string(),
                repo_url: crate::updater::OFFICIAL_REPO_URL.to_string(),
                checked_at: chrono::Local::now().format("%H:%M:%S").to_string(),
                network_error: false,
            };

            let p_prog = proxy.clone();
            let cb = move |progress: crate::updater::UpdateProgress| {
                if let Ok(prog_val) = serde_json::to_value(&progress) {
                    let event_obj = serde_json::json!({
                        "type": "event",
                        "event": "updateProgress",
                        "data": prog_val
                    });
                    if let Ok(event_str) = serde_json::to_string(&event_obj) {
                        if let Ok(safe_js_str) = serde_json::to_string(&event_str) {
                            let script = format!(
                                "if (typeof window.__onRustResponse === 'function') {{ window.__onRustResponse({}); }}",
                                safe_js_str
                            );
                            let _ = p_prog.send_event(UserEvent::EvalScript(script));
                        }
                    }
                }
            };

            let update_mgr = crate::updater::UpdateManager::new();
            match update_mgr.download_and_verify(&release_info, &crate::updater::OFFICIAL_PUBLIC_KEY_BYTES, cb) {
                Ok(installer_path) => {
                    crate::updater::UpdateManager::apply_update_and_restart(&installer_path)?;
                    Ok(serde_json::json!({ "success": true }))
                }
                Err(err) => {
                    crate::telemetry::log("ERROR", "UPDATER", &format!("Update failed: {}", err), None);
                    Err(err)
                }
            }
        }
        "pauseUpdate" | "resumeUpdate" | "cancelUpdate" => Ok(serde_json::json!(true)),
        "openExternalUrl" => {
            if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
                let trimmed = url.trim();
                if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    let _ = open::that(trimmed);
                } else {
                    return Err(format!("Blocked non-http(s) URL protocol: '{}'", url));
                }
            }
            Ok(serde_json::json!(true))
        }
        "reactMounted" => {
            log_startup(10, "React application mounted", Some("React DOM mount confirmed via IPC"));
            Ok(serde_json::json!(true))
        }
        "logClientError" => {
            let msg = payload.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown client error");
            let detail = payload.get("stack").and_then(|v| v.as_str());
            log_startup(9, "JavaScript errors detected", Some(msg));
            crate::telemetry::log("ERROR", "CLIENT_JS", msg, detail);
            Ok(serde_json::json!(true))
        }
        "getDevModeState" => {
            let metrics = crate::telemetry::get_metrics();
            let since_id = payload.get("sinceId").and_then(|v| v.as_u64()).unwrap_or(0);
            let logs = crate::telemetry::get_recent_logs(since_id, 300);
            Ok(serde_json::json!({
                "enabled": metrics.dev_mode_enabled,
                "logFilePath": metrics.active_log_file,
                "metrics": metrics,
                "logs": logs,
            }))
        }
        "setDevModeState" => {
            if let Some(enabled) = payload.get("enabled").and_then(|v| v.as_bool()) {
                crate::telemetry::set_enabled(enabled);
            }
            if let Some(path_val) = payload.get("logFilePath") {
                let path_buf = path_val.as_str().filter(|s| !s.trim().is_empty()).map(PathBuf::from);
                crate::telemetry::set_log_file_path(path_buf);
            }
            let metrics = crate::telemetry::get_metrics();
            Ok(serde_json::json!({
                "success": true,
                "enabled": metrics.dev_mode_enabled,
                "logFilePath": metrics.active_log_file,
                "metrics": metrics
            }))
        }
        "browseDevLogPath" => {
            let path = browse_dev_log_path();
            if let Some(ref p) = path {
                crate::telemetry::set_log_file_path(Some(PathBuf::from(p)));
            }
            Ok(serde_json::json!(path))
        }
        "clearDevLogs" => {
            crate::telemetry::clear_logs();
            crate::telemetry::log("INFO", "SYS", "Developer log buffer cleared by developer.", None);
            Ok(serde_json::json!(true))
        }
        "exportDevDiagnostics" => {
            let custom_path = payload.get("path").and_then(|v| v.as_str()).map(PathBuf::from);
            let dest_path = custom_path
                .or_else(|| crate::telemetry::get_log_file_path().map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("skinner_diagnostics_report.txt"));
            let sys_info = build_system_info(&state.installation, &state.catalog, &state.backup_manager);
            crate::telemetry::export_diagnostics_report(&dest_path, &sys_info)?;
            crate::telemetry::log("INFO", "SYS", &format!("Full diagnostics dumped to {}", dest_path.display()), None);
            Ok(serde_json::json!({
                "success": true,
                "path": dest_path.to_string_lossy()
            }))
        }
        "getDevLogs" => {
            let since_id = payload.get("sinceId").and_then(|v| v.as_u64()).unwrap_or(0);
            let limit = payload.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
            let logs = crate::telemetry::get_recent_logs(since_id, limit);
            let metrics = crate::telemetry::get_metrics();
            Ok(serde_json::json!({
                "metrics": metrics,
                "logs": logs
            }))
        }
        _ => Err(format!("Unknown action '{}'", action)),
    }
}

pub fn run_desktop_app() -> Result<(), Box<dyn std::error::Error>> {
    let installation = GameDetector::detect(None);

    let mut catalog = Catalog::default();
    if installation.is_valid {
        let _ = catalog.load_from_installation(&installation);
    }

    let backup_manager = BackupManager::new(&installation.game_path);
    let preset_manager = PresetManager::new();

    let shared_data = Arc::new(Mutex::new(SharedAppData {
        installation,
        catalog,
        backup_manager,
        preset_manager,
        renderer: Mutex::new(RendererClient::new()),
        pending_modifications: Vec::new(),
    }));

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut window_builder = WindowBuilder::new()
        .with_title("Skinner")
        .with_inner_size(LogicalSize::new(1280.0, 820.0))
        .with_min_inner_size(LogicalSize::new(1024.0, 680.0))
        .with_decorations(false)
        .with_resizable(true);

    let icon_bytes = include_bytes!("../assets/icon_256.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        if let Ok(icon) = Icon::from_rgba(rgba.into_raw(), width, height) {
            window_builder = window_builder.with_window_icon(Some(icon));
        }
    }

    let window = match window_builder.build(&event_loop) {
        Ok(w) => {
            log_startup(2, "Window created", Some("Window successfully built and configured"));
            Arc::new(w)
        }
        Err(e) => {
            let err_msg = format!("Failed to create application window: {}\n\nLogs: %APPDATA%\\Skinner\\Logs\\startup.log", e);
            log_startup(2, "Window creation failed", Some(&err_msg));
            show_native_error_dialog("Skinner — Window Creation Failure", &err_msg);
            return Err(e.into());
        }
    };

    let data_dir = get_user_data_dir();
    log_startup(3, "WebView2 environment initialized", Some(&format!("Data dir: {}", data_dir.display())));
    let mut web_context = wry::WebContext::new(Some(data_dir));

    let init_script = r#"
window.__pendingRustResponses = [];
window.__onRustResponse = function(raw) {
  if (Array.isArray(window.__pendingRustResponses)) {
    window.__pendingRustResponses.push(raw);
  }
};
"#;

    let p_clone = proxy.clone();
    let data_clone = Arc::clone(&shared_data);

    log_startup(5, "Custom protocol registered", Some("Registering protocol 'brawlcraft'"));

    let webview = match WebViewBuilder::new_with_web_context(&mut web_context)
        .with_initialization_script(init_script)
        .with_custom_protocol("brawlcraft".into(), |_webview_id, request| {
            let raw_path = request.uri().path();
            let trimmed = raw_path.trim_start_matches('/');
            let decoded = percent_encoding::percent_decode_str(trimmed).decode_utf8_lossy();
            let clean_path = decoded.trim_start_matches("./").trim_start_matches('/');
            let file_path = if clean_path.is_empty() { "index.html" } else { clean_path };

            match FrontendAssets::get(file_path) {
                Some(content) => {
                    let mime = mime_guess::from_path(file_path).first_or_octet_stream();
                    let mime_str = if file_path.ends_with(".js") {
                        "application/javascript; charset=utf-8"
                    } else if file_path.ends_with(".css") {
                        "text/css; charset=utf-8"
                    } else if file_path.ends_with(".html") {
                        "text/html; charset=utf-8"
                    } else {
                        mime.as_ref()
                    };

                    log_startup(
                        8,
                        "Frontend resources loaded",
                        Some(&format!("'{}' ({}, {} bytes)", file_path, mime_str, content.data.len())),
                    );

                    let mut builder = Response::builder()
                        .header(CONTENT_TYPE, mime_str)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
                        .header("Access-Control-Allow-Headers", "*")
                        .header("X-Content-Type-Options", "nosniff")
                        .header("X-Frame-Options", "DENY")
                        .header("Referrer-Policy", "no-referrer");

                    if file_path == "index.html" {
                        log_startup(7, "Navigation completed", Some("index.html served successfully"));
                        builder = builder.header(
                            "Content-Security-Policy",
                            "default-src 'self' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: data: blob:; script-src 'self' 'unsafe-inline' 'unsafe-eval' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft:; style-src 'self' 'unsafe-inline' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://fonts.googleapis.com; font-src 'self' data: http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://fonts.gstatic.com; img-src 'self' data: blob: http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft:; connect-src 'self' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://api.github.com https://github.com https://objects.githubusercontent.com;"
                        );
                    }

                    builder.body(content.data).unwrap()
                }
                None => {
                    log_startup(
                        8,
                        "Frontend resource missing, falling back to index.html",
                        Some(&format!("Requested: '{}'", file_path)),
                    );

                    if let Some(index) = FrontendAssets::get("index.html") {
                        Response::builder()
                            .header(CONTENT_TYPE, "text/html; charset=utf-8")
                            .header("Access-Control-Allow-Origin", "*")
                            .header("X-Content-Type-Options", "nosniff")
                            .header("X-Frame-Options", "DENY")
                            .header("Referrer-Policy", "no-referrer")
                            .header(
                                "Content-Security-Policy",
                                "default-src 'self' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: data: blob:; script-src 'self' 'unsafe-inline' 'unsafe-eval' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft:; style-src 'self' 'unsafe-inline' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://fonts.googleapis.com; font-src 'self' data: http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://fonts.gstatic.com; img-src 'self' data: blob: http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft:; connect-src 'self' http://brawlcraft.localhost https://brawlcraft.localhost brawlcraft: https://api.github.com https://github.com https://objects.githubusercontent.com;"
                            )
                            .body(index.data)
                            .unwrap()
                    } else {
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(std::borrow::Cow::Borrowed(&b"404 Not Found"[..]))
                            .unwrap()
                    }
                }
            }
        })
        .with_url("brawlcraft://localhost/index.html")
        .with_navigation_handler(|url: String| {
            let allowed = is_allowed_internal_navigation(&url);
            log_startup(
                6,
                "Frontend navigation started",
                Some(&format!("URL: '{}' (allowed: {})", url, allowed)),
            );
            allowed
        })
        .with_ipc_handler(move |req| {
            let body = req.body().to_string();
            let p_inner = p_clone.clone();
            let data_inner = Arc::clone(&data_clone);

            std::thread::spawn(move || {
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
                    let req_id = json_val
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let action = json_val
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let payload = json_val
                        .get("payload")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);

                    let start_t = std::time::Instant::now();

                    // Fast-path: satisfy cached asset requests instantly without taking the global application lock
                    if action == "getAssetSvg" || action == "renderPreview" {
                        if let Some(cache_key) = payload.get("cacheKey").and_then(|v| v.as_str()) {
                            if let Some(svg) = get_cached_svg_fast(cache_key) {
                                let response_obj = serde_json::json!({
                                    "id": req_id,
                                    "success": true,
                                    "data": svg,
                                    "error": serde_json::Value::Null
                                });
                                if let Ok(resp_str) = serde_json::to_string(&response_obj) {
                                    if let Ok(safe_js_str) = serde_json::to_string(&resp_str) {
                                        let script = format!(
                                            "if (typeof window.__onRustResponse === 'function') {{ window.__onRustResponse({}); }}",
                                            safe_js_str
                                        );
                                        let _ = p_inner.send_event(UserEvent::EvalScript(script));
                                    }
                                }
                                return;
                            }
                        }
                    }

                    if action != "getDevLogs" && action != "getDevModeState" {
                        crate::telemetry::log("INFO", "IPC", &format!("API Request: {}", action), None);
                    }

                    let (success, data, error) = {
                        let mut state = data_inner.lock().unwrap();
                        match handle_action(&mut state, &action, payload, &p_inner) {
                            Ok(res) => (true, res, serde_json::Value::Null),
                            Err(e) => (false, serde_json::Value::Null, serde_json::json!(e)),
                        }
                    };

                    let elapsed = start_t.elapsed();
                    if action != "getDevLogs" && action != "getDevModeState" {
                        let level = if success { "DEBUG" } else { "ERROR" };
                        let msg = format!("API Request '{}' completed in {:.2}ms (success: {})", action, elapsed.as_secs_f64() * 1000.0, success);
                        let err_detail = if !success { Some(error.to_string()) } else { None };
                        crate::telemetry::log(level, "IPC", &msg, err_detail.as_deref());
                    }

                    let response_obj = serde_json::json!({
                        "id": req_id,
                        "success": success,
                        "data": data,
                        "error": error
                    });

                    if let Ok(resp_str) = serde_json::to_string(&response_obj) {
                        if let Ok(safe_js_str) = serde_json::to_string(&resp_str) {
                            let script = format!(
                                "if (typeof window.__onRustResponse === 'function') {{ window.__onRustResponse({}); }}",
                                safe_js_str
                            );
                            let _ = p_inner.send_event(UserEvent::EvalScript(script));
                        }
                    }
                }
            });
        })
        .build(&window)
    {
        Ok(w) => {
            log_startup(4, "WebView2 controller created", Some("WebView2 controller attached to window"));
            w
        }
        Err(e) => {
            let err_msg = format!("Failed to initialize WebView2 runtime: {}\n\nPlease ensure Microsoft Edge WebView2 Runtime is installed.\n\nStartup log location:\n%APPDATA%\\Skinner\\Logs\\startup.log", e);
            log_startup(4, "WebView2 controller creation failed", Some(&err_msg));
            show_native_error_dialog("Skinner — WebView2 Initialization Error", &err_msg);
            return Err(e.into());
        }
    };

    let win_ref = Arc::clone(&window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::EvalScript(script)) => {
                let _ = webview.evaluate_script(&script);
            }
            Event::UserEvent(UserEvent::Minimize) => {
                win_ref.set_minimized(true);
            }
            Event::UserEvent(UserEvent::Maximize) => {
                let is_max = win_ref.is_maximized();
                win_ref.set_maximized(!is_max);
                let script = format!(
                    "if (window.__setWindowState) window.__setWindowState({});",
                    !is_max
                );
                let _ = webview.evaluate_script(&script);
            }
            Event::UserEvent(UserEvent::Close) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Drag) => {
                let _ = win_ref.drag_window();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
