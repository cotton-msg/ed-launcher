use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const HOST_URL: &str = "http://46.42.3.254:25566";
const FORGE_VERSION: &str = "1.20.1-47.4.20";
const VANILLA_VERSION: &str = "1.20.1";

fn base_dir() -> PathBuf {
    PathBuf::from("C:\\edlauncher")
}

fn java_dir() -> PathBuf { base_dir().join("java") }
fn version_dir() -> PathBuf { base_dir().join("version") }
fn java_exe() -> PathBuf { java_dir().join("bin").join("java.exe") }
fn forge_json_path() -> PathBuf {
    let name = format!("{}-forge-{}", VANILLA_VERSION, FORGE_VERSION.split('-').nth(1).unwrap_or("47.4.20"));
    version_dir().join("versions").join(&name).join(format!("{}.json", name))
}
fn client_jar_path() -> PathBuf {
    let forge_suffix = FORGE_VERSION.split('-').nth(1).unwrap_or("47.4.20");
    let name = format!("{}-forge-{}", VANILLA_VERSION, forge_suffix);
    version_dir().join("versions").join(&name).join(format!("{}-{}.jar", VANILLA_VERSION, forge_suffix))
}
fn libs_dir() -> PathBuf { version_dir().join("libraries") }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerStatus {
    pub online: bool,
    pub players: u32,
    pub max_players: u32,
    pub version: String,
    pub ping_ms: u32,
    pub tps: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LauncherSettings {
    pub ram_gb: f64,
    pub java_version: String,
    pub window_width: String,
    pub window_height: String,
    pub fullscreen: bool,
    pub vsync: bool,
    pub keep_open: bool,
    pub debug_info: bool,
    pub last_nickname: String,
    pub game_path: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            ram_gb: 4.0,
            java_version: "Java 17".to_string(),
            window_width: "1280".to_string(),
            window_height: "720".to_string(),
            fullscreen: false,
            vsync: true,
            keep_open: false,
            debug_info: false,
            last_nickname: String::new(),
            game_path: String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DownloadProgress {
    pub file: String,
    pub progress: f64,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub speed_mb_s: f64,
}

pub struct AppState {
    settings: Mutex<LauncherSettings>,
}

fn get_app_data_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("failed to get app data dir");
    fs::create_dir_all(&dir).expect("failed to create app data dir");
    dir
}

fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    get_app_data_dir(app).join("settings.json")
}

async fn download_archive(app: &tauri::AppHandle, label: &str, url: &str, out_path: &PathBuf) -> Result<(), String> {
    use futures_util::StreamExt;

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let known_size: f64 = match label {
        "java.zip" => 42.0,
        "version.zip" => 1900.0,
        _ => 0.0,
    };

    let _ = app.emit("download-progress", &DownloadProgress {
        file: format!("Downloading {}...", label),
        progress: 0.0,
        downloaded_mb: 0.0,
        total_mb: known_size,
        speed_mb_s: 0.0,
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(3600))
        .user_agent("GrandEdenLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(url).send().await.map_err(|e| format!("Failed to download {}: {}", label, e))?;
    if !response.status().is_success() {
        return Err(format!("Failed to download {}: HTTP {}", label, response.status()));
    }

    let total_size = response.content_length().unwrap_or(0) as f64;
    let total_mb = if total_size > 0.0 { total_size / 1_048_576.0 } else { known_size };
    let mut downloaded: f64 = 0.0;
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(out_path).map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as f64;

        if total_size > 0.0 && downloaded as usize % (512 * 1024) < chunk.len() {
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let _ = app.emit("download-progress", &DownloadProgress {
                file: format!("Downloading {}...", label),
                progress: (downloaded / total_size) * 100.0,
                downloaded_mb: downloaded / 1_048_576.0,
                total_mb,
                speed_mb_s: (downloaded / 1_048_576.0) / elapsed,
            });
        }
    }

    Ok(())
}

fn validate_zip(zip_path: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Cannot open {}: {}", zip_path.display(), e))?;
    zip::ZipArchive::new(file).map_err(|e| format!("Corrupted zip {}: {}", zip_path.display(), e))?;
    Ok(())
}

fn extract_zip(app: &tauri::AppHandle, zip_path: &Path, dest: &Path) -> Result<(), String> {
    let _ = app.emit("download-progress", &DownloadProgress {
        file: format!("Extracting {}...", zip_path.file_name().unwrap_or_default().to_string_lossy()),
        progress: 0.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    });

    let zip_file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip {}: {}", zip_path.display(), e))?;
    let total = archive.len();

    for i in 0..total {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let out_path = dest.join(&name);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            out_file.write_all(&buf).map_err(|e| e.to_string())?;
        }

        if i % 100 == 0 || i == total - 1 {
            let pct = if total > 0 { (i as f64 / total as f64) * 100.0 } else { 100.0 };
            let _ = app.emit("download-progress", &DownloadProgress {
                file: format!("Extracting {}... ({}/{})", zip_path.file_name().unwrap_or_default().to_string_lossy(), i + 1, total),
                progress: pct,
                downloaded_mb: (i + 1) as f64,
                total_mb: total as f64,
                speed_mb_s: 0.0,
            });
        }
    }

    Ok(())
}

#[tauri::command]
async fn download_game(app: tauri::AppHandle, _game_path: String) -> Result<bool, String> {
    let bd = base_dir();
    fs::create_dir_all(&bd).map_err(|e| e.to_string())?;

    let java_zip = bd.join("java.zip");
    let version_zip = bd.join("version.zip");

    if !java_exe().exists() {
        if !java_zip.exists() {
            download_archive(&app, "java.zip", &format!("{}/java.zip", HOST_URL), &java_zip).await?;
        }
        validate_zip(&java_zip)?;
        extract_zip(&app, &java_zip, &bd)?;
    }

    if !forge_json_path().exists() || !client_jar_path().exists() {
        let mut tries = 0;
        loop {
            if !version_zip.exists() || tries > 0 {
                if version_zip.exists() {
                    let _ = fs::remove_file(&version_zip);
                }
                download_archive(&app, "version.zip", &format!("{}/version.zip", HOST_URL), &version_zip).await?;
            }
            match validate_zip(&version_zip) {
                Ok(()) => break,
                Err(e) => {
                    if tries >= 2 {
                        return Err(format!("{} (after 3 download attempts)", e));
                    }
                    tries += 1;
                    let _ = fs::remove_file(&version_zip);
                    let _ = app.emit("download-progress", &DownloadProgress {
                        file: format!("Download incomplete, retrying ({}/3)...", tries + 1),
                        progress: 0.0, downloaded_mb: 0.0, total_mb: 0.0, speed_mb_s: 0.0,
                    });
                }
            }
        }
        extract_zip(&app, &version_zip, &bd)?;
    }

    if !java_exe().exists() {
        return Err("Java not found after extraction.".to_string());
    }
    if !forge_json_path().exists() {
        return Err("Forge files not found. Check HOST_URL.".to_string());
    }

    let _ = app.emit("download-progress", &DownloadProgress {
        file: "Ready to launch".to_string(),
        progress: 100.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    });

    Ok(true)
}

fn is_lib_allowed(lib: &serde_json::Value) -> bool {
    let rules = lib.get("rules").and_then(|r| r.as_array());
    let Some(rules) = rules else { return true };
    let mut allowed = false;
    for rule in rules {
        let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("allow");
        let os_info = rule.get("os");
        if let Some(os) = os_info {
            if action == "allow" && os.get("name").and_then(|v| v.as_str()) == Some("windows") {
                allowed = true;
            }
            if action == "disallow" && os.get("name").and_then(|v| v.as_str()) == Some("windows") {
                return false;
            }
        } else if action == "allow" {
            allowed = true;
        }
    }
    allowed
}

fn build_launch_args(nickname: &str, ram_gb: f64, width: &str, height: &str) -> Result<Vec<String>, String> {
    let ram = (ram_gb * 1024.0) as u32;
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mc_versions_dir = version_dir().join("versions").join(format!("{}-forge-{}", VANILLA_VERSION, FORGE_VERSION.split('-').nth(1).unwrap_or("47.4.20")));
    let forge_json_str = fs::read_to_string(&forge_json_path()).map_err(|e| format!("Failed to read forge JSON: {}", e))?;
    let forge_json: serde_json::Value = serde_json::from_str(&forge_json_str).map_err(|e| format!("Failed to parse forge JSON: {}", e))?;

    let vanilla_json_path = version_dir().join("versions").join(VANILLA_VERSION).join(format!("{}.json", VANILLA_VERSION));
    let asset_index_id = if vanilla_json_path.exists() {
        if let Ok(s) = fs::read_to_string(&vanilla_json_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                v.get("assetIndex").and_then(|a| a.get("id")).and_then(|i| i.as_str()).unwrap_or("5").to_string()
            } else { "5".to_string() }
        } else { "5".to_string() }
    } else { "5".to_string() };

    let mut classpath: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let client_jar = client_jar_path();
    if client_jar.exists() {
        let p = client_jar.to_string_lossy().to_string();
        seen.insert(p.clone());
        classpath.push(p);
    }

    for lib in forge_json.get("libraries").and_then(|v| v.as_array()).unwrap_or(&vec![]) {
        let art = lib.get("downloads").and_then(|d| d.get("artifact"));
        if let Some(art) = art {
            if let Some(path) = art.get("path").and_then(|v| v.as_str()) {
                if !is_lib_allowed(lib) { continue; }
                let p = libs_dir().join(path).to_string_lossy().to_string();
                if seen.insert(p.clone()) && Path::new(&p).exists() {
                    classpath.push(p);
                }
            }
        }
    }

    if vanilla_json_path.exists() {
        if let Ok(s) = fs::read_to_string(&vanilla_json_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                for lib in v.get("libraries").and_then(|v| v.as_array()).unwrap_or(&vec![]) {
                    let art = lib.get("downloads").and_then(|d| d.get("artifact"));
                    if let Some(art) = art {
                        if let Some(path) = art.get("path").and_then(|v| v.as_str()) {
                            if !is_lib_allowed(lib) { continue; }
                            let p = libs_dir().join(path).to_string_lossy().to_string();
                            if seen.insert(p.clone()) && Path::new(&p).exists() {
                                classpath.push(p);
                            }
                        }
                    }
                }
            }
        }
    }

    let cp_str = classpath.join(sep);
    let libs_str = libs_dir().to_string_lossy().to_string();
    let mc_versions_str = mc_versions_dir.to_string_lossy().to_string();
    let version_dir_str = version_dir().to_string_lossy().to_string();
    let assets_dir_str = version_dir().join("assets").to_string_lossy().to_string();

    let forge_version = FORGE_VERSION;

    let mut args: Vec<String> = vec![
        format!("-Xmx{}m", ram),
        format!("-Xms{}m", ram / 2),
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=50".to_string(),
    ];

    let mut replace_next: Option<&str> = None;
    if let Some(jvm_args) = forge_json.get("arguments").and_then(|a| a.get("jvm")).and_then(|a| a.as_array()) {
        for arg in jvm_args {
            if let Some(s) = arg.as_str() {
                if replace_next == Some("cp") {
                    args.push(cp_str.clone());
                    replace_next = None;
                    continue;
                }
                if s == "-cp" {
                    args.push("-cp".to_string());
                    replace_next = Some("cp");
                    continue;
                }
                if s == "-p" {
                    args.push("-p".to_string());
                    continue;
                }
                let substituted = s
                    .replace("${launcher_name}", "EDLauncher")
                    .replace("${launcher_version}", "1.0")
                    .replace("${version_name}", forge_version)
                    .replace("${library_directory}", &libs_str)
                    .replace("${classpath_separator}", sep)
                    .replace("${version_classpath_separator}", sep)
                    .replace("${natives_directory}", &mc_versions_str)
                    .replace("${classpath}", &cp_str);
                let final_arg = if substituted.starts_with("-DignoreList=") {
                    format!("{},ForgeAutoRenamingTool", substituted)
                } else {
                    substituted
                };
                args.push(final_arg);
            } else if let Some(obj) = arg.as_object() {
                let mut allowed = true;
                if let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) {
                    allowed = false;
                    for rule in rules {
                        let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("allow");
                        if rule.get("features").is_some() { continue; }
                        if let Some(os) = rule.get("os") {
                            let os_name = os.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            if action == "allow" && os_name == "windows" { allowed = true; }
                            if action == "disallow" && os_name == "windows" { allowed = false; }
                        } else if action == "allow" { allowed = true; }
                    }
                }
                if allowed {
                    if let Some(values) = obj.get("value") {
                        if let Some(arr) = values.as_array() {
                            for v in arr {
                                if let Some(s) = v.as_str() {
                                    args.push(s.replace("${library_directory}", &libs_str));
                                }
                            }
                        } else if let Some(s) = values.as_str() {
                            args.push(s.replace("${library_directory}", &libs_str));
                        }
                    }
                }
            }
        }
    }

    let main_class = forge_json.get("mainClass").and_then(|v| v.as_str()).unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher");
    args.push(main_class.to_string());

    if let Some(game_args) = forge_json.get("arguments").and_then(|a| a.get("game")).and_then(|a| a.as_array()) {
        for arg in game_args {
            if let Some(s) = arg.as_str() {
                let substituted = s
                    .replace("${auth_player_name}", nickname)
                    .replace("${auth_session}", "0")
                    .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
                    .replace("${auth_access_token}", "0")
                    .replace("${clientid}", "")
                    .replace("${auth_xuid}", "")
                    .replace("${version_name}", forge_version)
                    .replace("${game_directory}", &version_dir_str)
                    .replace("${assets_root}", &assets_dir_str)
                    .replace("${assets_index_name}", &asset_index_id)
                    .replace("${user_type}", "offline")
                    .replace("${user_properties}", "{}")
                    .replace("${version_type}", "Forge")
                    .replace("${resolution_width}", width)
                    .replace("${resolution_height}", height);
                args.push(substituted);
            }
        }
    }

    args.push("--width".to_string());
    args.push(width.to_string());
    args.push("--height".to_string());
    args.push(height.to_string());

    Ok(args)
}

#[tauri::command]
async fn launch_game(
    _app: tauri::AppHandle,
    nickname: String,
    settings: LauncherSettings,
) -> Result<bool, String> {
    if !java_exe().exists() {
        return Err("Java not found. Download game files first.".to_string());
    }
    if !forge_json_path().exists() {
        return Err("Forge files not found. Download game files first.".to_string());
    }

    let mc_versions = version_dir().join("versions").join(format!("{}-forge-{}", VANILLA_VERSION, FORGE_VERSION.split('-').nth(1).unwrap_or("47.4.20")));
    let args = build_launch_args(&nickname, settings.ram_gb, &settings.window_width, &settings.window_height)?;

    let debug_path = mc_versions.join("debug_args.txt");
    let debug_content = format!("{}\n", args.join(" "));
    let _ = fs::write(&debug_path, &debug_content);

    let log_path = mc_versions.join("launcher.log");
    let log_file = fs::File::create(&log_path).map_err(|e| format!("Failed to create log: {}", e))?;

    let mut child = std::process::Command::new(java_exe())
        .args(&args)
        .current_dir(&mc_versions)
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file)
        .spawn()
        .map_err(|e| format!("Failed to launch game: {}", e))?;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    match child.try_wait() {
        Ok(Some(status)) => {
            let log_content = fs::read_to_string(&log_path).unwrap_or_default();
            let tail: String = log_content.lines().rev().take(30).collect::<Vec<_>>().join("\n");
            return Err(format!("Game exited immediately with status: {}. Last log lines:\n{}", status, tail));
        }
        Ok(None) => {}
        Err(e) => return Err(format!("Failed to check game status: {}", e)),
    }

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(true)
}

#[tauri::command]
async fn check_server_status() -> Result<ServerStatus, String> {
    Ok(ServerStatus {
        online: false,
        players: 0,
        max_players: 500,
        version: "1.20.1".to_string(),
        ping_ms: 0,
        tps: 0.0,
    })
}

#[tauri::command]
fn load_settings(app: tauri::AppHandle) -> LauncherSettings {
    let path = get_settings_path(&app);
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        LauncherSettings::default()
    }
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: LauncherSettings) -> Result<(), String> {
    let path = get_settings_path(&app);
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    if let Ok(mut s) = state.settings.lock() {
        *s = settings;
    }
    Ok(())
}

#[tauri::command]
fn window_minimize(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn window_maximize(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            window.unmaximize().map_err(|e| e.to_string())?;
        } else {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn window_close(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn window_is_maximized(_app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(window) = _app.get_webview_window("main") {
        return Ok(window.is_maximized().unwrap_or(false));
    }
    Ok(false)
}

#[tauri::command]
async fn select_game_folder(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select Game Folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder.map(|p| p.to_string()).unwrap_or_default());
        });
    rx.await.map_err(|e| format!("Folder picker cancelled: {}", e))
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FileInfo {
    path: String,
    size: u64,
}

fn scan_directory(dir: &Path, base: &Path) -> Vec<FileInfo> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(scan_directory(&path, base));
            } else if let Ok(meta) = fs::metadata(&path) {
                let relative = path.strip_prefix(base).unwrap_or(&path);
                files.push(FileInfo {
                    path: relative.to_string_lossy().to_string(),
                    size: meta.len(),
                });
            }
        }
    }
    files
}

#[tauri::command]
async fn report_game_inventory(nickname: String) -> Result<bool, String> {
    let vd = version_dir();
    if !vd.exists() {
        return Ok(false);
    }

    let files = tokio::task::spawn_blocking(move || scan_directory(&vd, &vd))
        .await
        .map_err(|e| e.to_string())?;

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let file_count = files.len();

    let payload = serde_json::json!({
        "nickname": nickname,
        "files": files,
        "total_size": total_size,
        "file_count": file_count,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/inventory", HOST_URL);
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Server request failed: {}", e))?;

    if resp.status().is_success() {
        Ok(true)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Server error {}: {}", status, body))
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            settings: Mutex::new(LauncherSettings::default()),
        })
        .invoke_handler(tauri::generate_handler![
            check_server_status,
            load_settings,
            save_settings,
            download_game,
            launch_game,
            window_minimize,
            window_maximize,
            window_close,
            window_is_maximized,
            select_game_folder,
            report_game_inventory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
