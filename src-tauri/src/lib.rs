use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const SUPABASE_URL: &str = "https://wziuqwtunlovtyxleles.supabase.co";
const SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Ind6aXVxd3R1bmxvdnR5eGxlbGVzIiwicm9sZSI6ImFub24iLCJpYXQiOjE3ODM2OTI0MTYsImV4cCI6MjA5OTI2ODQxNn0.4mVMKOpLhx9_bfwBRnZAfXgiAJAmockvevMI5FkPRXg";
const SUPABASE_BUCKET: &str = "game-files";

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
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    get_app_data_dir(app).join("settings.json")
}

const MC_SERVER_HOST: &str = "c16.play2go.cloud";
const MC_SERVER_PORT: u16 = 2003;

fn mc_varint(value: i32) -> Vec<u8> {
    let mut result = Vec::new();
    let mut val = value as u32;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        result.push(byte);
        if val == 0 {
            break;
        }
    }
    result
}

fn mc_read_varint(data: &[u8], offset: &mut usize) -> i32 {
    let mut result: i32 = 0;
    let mut shift = 0;
    loop {
        if *offset >= data.len() {
            break;
        }
        let byte = data[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as i32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
    }
    result
}

fn mc_string(s: &str) -> Vec<u8> {
    let mut result = mc_varint(s.len() as i32);
    result.extend_from_slice(s.as_bytes());
    result
}

fn mc_packet(id: i32, payload: &[u8]) -> Vec<u8> {
    let mut result = mc_varint(payload.len() as i32 + 1);
    result.extend_from_slice(&mc_varint(id));
    result.extend_from_slice(payload);
    result
}

async fn ping_mc_server() -> Result<ServerStatus, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    let addr = format!("{}:{}", MC_SERVER_HOST, MC_SERVER_PORT);
    let connect_timeout = std::time::Duration::from_secs(5);
    let io_timeout = std::time::Duration::from_secs(5);

    let mut stream = timeout(connect_timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| "Connection timeout".to_string())?
        .map_err(|e| format!("Connection failed: {}", e))?;

    // Handshake: protocol 763 (1.20.1), next_state=1 (status)
    let mut payload = Vec::new();
    payload.extend_from_slice(&mc_varint(763));
    payload.extend_from_slice(&mc_string(MC_SERVER_HOST));
    payload.extend_from_slice(&MC_SERVER_PORT.to_be_bytes());
    payload.extend_from_slice(&mc_varint(1));
    let handshake = mc_packet(0x00, &payload);

    timeout(io_timeout, stream.write_all(&handshake))
        .await
        .map_err(|_| "Write timeout".to_string())?
        .map_err(|e| e.to_string())?;

    // Status request
    let status_req = mc_packet(0x00, &[]);
    timeout(io_timeout, stream.write_all(&status_req))
        .await
        .map_err(|_| "Write timeout".to_string())?
        .map_err(|e| e.to_string())?;

    // Read response
    let start = std::time::Instant::now();
    let mut buffer = vec![0u8; 4096];
    let n = timeout(io_timeout, stream.read(&mut buffer))
        .await
        .map_err(|_| "Read timeout".to_string())?
        .map_err(|e| e.to_string())?;
    let ping_ms = start.elapsed().as_millis() as u32;

    if n == 0 {
        return Err("Empty response".to_string());
    }

    let mut offset = 0;
    let _length = mc_read_varint(&buffer, &mut offset);
    let _packet_id = mc_read_varint(&buffer, &mut offset);
    let _json_length = mc_read_varint(&buffer, &mut offset);

    let json_str = std::str::from_utf8(&buffer[offset..n])
        .map_err(|e| format!("Invalid UTF-8: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let players_online = json["players"]["online"].as_u64().unwrap_or(0) as u32;
    let max_players = json["players"]["max"].as_u64().unwrap_or(0) as u32;
    let version = json["version"]["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok(ServerStatus {
        online: true,
        players: players_online,
        max_players,
        version,
        ping_ms,
        tps: 20.0,
    })
}

#[tauri::command]
async fn check_server_status() -> Result<ServerStatus, String> {
    match ping_mc_server().await {
        Ok(status) => Ok(status),
        Err(_) => Ok(ServerStatus {
            online: false,
            players: 0,
            max_players: 500,
            version: "1.20.1".to_string(),
            ping_ms: 0,
            tps: 0.0,
        }),
    }
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

async fn download_file_with_progress(
    app: &tauri::AppHandle,
    label: &str,
    url: &str,
    out_path: &PathBuf,
) -> Result<(), String> {
    use futures_util::StreamExt;

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .user_agent("GrandEdenLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Failed to download {}: HTTP {}", label, response.status()));
    }

    let total_size = response.content_length().unwrap_or(0) as f64;
    let total_mb = if total_size > 0.0 { total_size / 1_048_576.0 } else { 0.0 };
    let mut downloaded: f64 = 0.0;
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(out_path).map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as f64;

        if total_size > 0.0 && downloaded as usize % (256 * 1024) < chunk.len() {
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let progress = DownloadProgress {
                file: label.to_string(),
                progress: (downloaded / total_size) * 100.0,
                downloaded_mb: downloaded / 1_048_576.0,
                total_mb,
                speed_mb_s: (downloaded / 1_048_576.0) / elapsed,
            };
            let _ = app.emit("download-progress", &progress);
        }
    }

    Ok(())
}

// Recursively list all files in a Supabase Storage folder
async fn list_supabase_files(prefix: &str) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .user_agent("GrandEdenLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let mut all_files = Vec::new();
    let mut folders_to_check = vec![prefix.to_string()];

    while let Some(folder) = folders_to_check.pop() {
        let url = format!("{}/storage/v1/object/list/{}", SUPABASE_URL, SUPABASE_BUCKET);

        let body = serde_json::json!({
            "prefix": folder,
            "limit": 1000,
            "offset": 0,
            "sortBy": { "column": "name", "order": "asc" }
        });

        let response = client
            .post(&url)
            .header("apikey", SUPABASE_ANON_KEY)
            .header("Authorization", format!("Bearer {}", SUPABASE_ANON_KEY))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("Supabase list failed: HTTP {}", response.status()));
        }

        let items: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;

        for item in &items {
            let name = item["name"].as_str().unwrap_or("");
            let id = item["id"].as_str();

            if id.is_none() && !name.is_empty() {
                // It's a folder
                folders_to_check.push(format!("{}{}/", folder, name));
            } else if !name.is_empty() {
                // It's a file
                all_files.push(format!("{}{}", folder, name));
            }
        }
    }

    Ok(all_files)
}

#[tauri::command]
async fn download_game(
    app: tauri::AppHandle,
    _game_path: String,
) -> Result<bool, String> {
    let game_dir = get_app_data_dir(&app).join("game");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    // Download Java from Supabase Storage
    let java_dir = game_dir.join("java");
    if !java_dir.exists() || !java_dir.join("bin").join("java.exe").exists() {
        let _ = app.emit("download-progress", &DownloadProgress {
            file: "Downloading Java...".to_string(),
            progress: 0.0,
            downloaded_mb: 0.0,
            total_mb: 0.0,
            speed_mb_s: 0.0,
        });

        let java_files = list_supabase_files("java/").await?;
        let total_files = java_files.len();

        for (index, file_path) in java_files.iter().enumerate() {
            let url = format!(
                "{}/storage/v1/object/public/{}/{}",
                SUPABASE_URL, SUPABASE_BUCKET, file_path
            );

            let local_path = game_dir.join(file_path);

            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            let label = format!("Java: {} ({}/{})", 
                file_path.split('/').last().unwrap_or("file"),
                index + 1,
                total_files
            );

            download_file_with_progress(&app, &label, &url, &local_path).await.ok();

            let progress = ((index + 1) as f64 / total_files as f64) * 100.0;
            let _ = app.emit("download-progress", &DownloadProgress {
                file: label,
                progress,
                downloaded_mb: (index + 1) as f64,
                total_mb: total_files as f64,
                speed_mb_s: 0.0,
            });
        }
    }

    // Download version folder from Supabase Storage
    let version_dir = game_dir.join("version");
    if !version_dir.exists() {
        let _ = app.emit("download-progress", &DownloadProgress {
            file: "Downloading game files...".to_string(),
            progress: 0.0,
            downloaded_mb: 0.0,
            total_mb: 0.0,
            speed_mb_s: 0.0,
        });

        let version_files = list_supabase_files("version/").await?;
        let total_files = version_files.len();

        for (index, file_path) in version_files.iter().enumerate() {
            let url = format!(
                "{}/storage/v1/object/public/{}/{}",
                SUPABASE_URL, SUPABASE_BUCKET, file_path
            );
            
            let local_path = game_dir.join(file_path);
            
            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent).ok();
            }

            let label = format!("Game: {} ({}/{})", 
                file_path.split('/').last().unwrap_or("file"),
                index + 1,
                total_files
            );

            download_file_with_progress(&app, &label, &url, &local_path).await.ok();

            let progress = ((index + 1) as f64 / total_files as f64) * 100.0;
            let _ = app.emit("download-progress", &DownloadProgress {
                file: label,
                progress,
                downloaded_mb: (index + 1) as f64,
                total_mb: total_files as f64,
                speed_mb_s: 0.0,
            });
        }
    }

    // Download Minecraft assets from Mojang CDN
    let mc_dir = dirs_next::home_dir().unwrap_or_default().join(".minecraft");
    let assets_dir = mc_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    let objects_dir = assets_dir.join("objects");
    if let Some(index_file) = find_asset_index(&indexes_dir) {
        let index = load_asset_index(&index_file)?;
        let missing = find_missing_assets(&objects_dir, &index);
        if !missing.is_empty() {
            let total = missing.len();
            let _ = app.emit("download-progress", &DownloadProgress {
                file: format!("Downloading {} assets...", total),
                progress: 0.0,
                downloaded_mb: 0.0,
                total_mb: 0.0,
                speed_mb_s: 0.0,
            });
            for (i, (hash, _size)) in missing.iter().enumerate() {
                let prefix = &hash[..2];
                let url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    prefix, hash
                );
                let path = objects_dir.join(prefix).join(hash);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                let label = format!("Asset {}/{}", i + 1, total);
                download_file_with_progress(&app, &label, &url, &path).await.ok();
                let progress = ((i + 1) as f64 / total as f64) * 100.0;
                let _ = app.emit("download-progress", &DownloadProgress {
                    file: label,
                    progress,
                    downloaded_mb: (i + 1) as f64,
                    total_mb: total as f64,
                    speed_mb_s: 0.0,
                });
            }
        }
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

fn find_asset_index(indexes_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let forge_index = indexes_dir.join("5.json");
    if forge_index.exists() {
        return Some(forge_index);
    }
    std::fs::read_dir(indexes_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|e| e.path())
}

fn load_asset_index(path: &std::path::Path) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn find_missing_assets(objects_dir: &std::path::Path, index: &serde_json::Value) -> Vec<(String, u64)> {
    let mut missing = Vec::new();
    if let Some(objects) = index.get("objects").and_then(|o| o.as_object()) {
        for (_name, entry) in objects {
            if let Some(hash) = entry.get("hash").and_then(|h| h.as_str()) {
                let size = entry.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                let prefix = &hash[..2];
                let path = objects_dir.join(prefix).join(hash);
                if !path.exists() {
                    missing.push((hash.to_string(), size));
                }
            }
        }
    }
    missing
}

fn find_java_exec(game_dir: &PathBuf) -> PathBuf {
    // 1. Check system Java first (downloaded may lack modules file)
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home).join("bin").join("java.exe");
        if p.exists() {
            return p;
        }
    }

    let candidates = [
        "C:\\Program Files\\Eclipse Adoptium",
        "C:\\Program Files\\Java",
        "C:\\Program Files\\Microsoft",
        "C:\\Program Files\\BellSoft",
        "C:\\Program Files\\Zulu",
    ];
    for base in &candidates {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let p = entry.path().join("bin").join("java.exe");
                if p.exists() {
                    return p;
                }
            }
        }
    }

    // 2. Fallback to downloaded Java
    game_dir.join("java").join("bin").join("java.exe")
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    nickname: String,
    settings: LauncherSettings,
) -> Result<bool, String> {
    let game_dir = get_app_data_dir(&app).join("game");
    let version_dir = game_dir.join("version");

    // Find Java: system first, then downloaded
    let java_exe = find_java_exec(&game_dir);

    if !java_exe.exists() {
        return Err("Java not found. Please download game files first.".to_string());
    }

    if !version_dir.exists() {
        return Err("Game files not found. Please download game files first.".to_string());
    }

    // Forge libraries from .minecraft
    let mc_libs = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".minecraft")
        .join("libraries");
    let libs_str = mc_libs.to_string_lossy().to_string();

    let sep = if cfg!(windows) { ";" } else { ":" };

    let mc_dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".minecraft");
    let mc_versions = mc_dir.join("versions");
    let forge_version_json = mc_versions.join("1.20.1-forge-47.4.20/1.20.1-forge-47.4.20.json");

    // Collect library artifact paths from a JSON value array
    fn collect_lib_paths(libraries: &[serde_json::Value], libs_str: &str) -> Vec<String> {
        let mut paths = Vec::new();
        for lib in libraries {
            if let Some(rules) = lib.get("rules").and_then(|v| v.as_array()) {
                let mut allowed = false;
                for rule in rules {
                    let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("allow");
                    if let Some(os) = rule.get("os") {
                        let os_name = os.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        if action == "allow" && os_name == std::env::consts::OS {
                            allowed = true;
                        }
                        if action == "disallow" && os_name == std::env::consts::OS {
                            allowed = false;
                        }
                    } else if action == "allow" {
                        allowed = true;
                    }
                }
                if !allowed {
                    continue;
                }
            }
            if let Some(downloads) = lib.get("downloads") {
                if let Some(artifact) = downloads.get("artifact") {
                    if let Some(path) = artifact.get("path").and_then(|v| v.as_str()) {
                        let jar_path = format!("{}/{}", libs_str, path);
                        if std::path::Path::new(&jar_path).exists() {
                            paths.push(jar_path);
                        }
                    }
                }
            }
        }
        paths
    }

    // Load the version JSON — read everything from it
    let version_json_str = fs::read_to_string(&forge_version_json)
        .map_err(|e| format!("Failed to read Forge version JSON: {}", e))?;
    let version_json: serde_json::Value = serde_json::from_str(&version_json_str)
        .map_err(|e| format!("Failed to parse Forge version JSON: {}", e))?;

    let version_name = "1.20.1-47.4.20";

    // Build classpath: libraries from JSON + client jar (no mods — Forge loads them from mods/ dir)
    let mut classpath = Vec::new();

    // Add client jar
    let client_jar = format!("{}/1.20.1-47.4.20.jar", version_dir.to_string_lossy());
    if std::path::Path::new(&client_jar).exists() {
        classpath.push(client_jar);
    }

    // Add libraries from JSON
    if let Some(libraries) = version_json.get("libraries").and_then(|v| v.as_array()) {
        let mut seen = std::collections::HashSet::new();
        for path in collect_lib_paths(libraries, &libs_str) {
            if seen.insert(path.clone()) {
                classpath.push(path);
            }
        }
    }

    let classpath_str = classpath.join(sep);

    // Parse JVM args from version JSON, substitute variables
    let ram_mb = (settings.ram_gb * 1024.0) as u32;

    let mut args: Vec<String> = vec![
        format!("-Xmx{}m", ram_mb),
        format!("-Xms{}m", ram_mb / 2),
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=50".to_string(),
    ];

    // Add JVM args from version JSON (let the JSON define everything)
    if let Some(jvm_args) = version_json.get("arguments").and_then(|a| a.get("jvm")).and_then(|a| a.as_array()) {
        let mut replace_next_cp = false;
        let mut replace_next_p = false;
        for arg in jvm_args {
            if let Some(s) = arg.as_str() {
                if replace_next_cp {
                    args.push(classpath_str.clone());
                    replace_next_cp = false;
                    continue;
                }
                if replace_next_p {
                    args.push(s
                        .replace("${library_directory}", &libs_str)
                        .replace("${classpath_separator}", sep)
                        .replace("${version_classpath_separator}", sep));
                    replace_next_p = false;
                    continue;
                }
                if s == "-cp" {
                    args.push("-cp".to_string());
                    replace_next_cp = true;
                    continue;
                }
                if s == "-p" {
                    args.push("-p".to_string());
                    replace_next_p = true;
                    continue;
                }
                let substituted = s
                    .replace("${launcher_name}", "GrandEdenLauncher")
                    .replace("${launcher_version}", "1.0.0")
                    .replace("${version_name}", version_name)
                    .replace("${library_directory}", &libs_str)
                    .replace("${classpath_separator}", sep)
                    .replace("${version_classpath_separator}", sep)
                    .replace("${natives_directory}", &version_dir.to_string_lossy())
                    .replace("${classpath}", &classpath_str);
                // Append ForgeAutoRenamingTool to ignoreList to prevent module conflict with ASM
                if substituted.starts_with("-DignoreList=") {
                    args.push(format!("{},ForgeAutoRenamingTool", substituted));
                } else {
                    args.push(substituted);
                }
            } else if let Some(obj) = arg.as_object() {
                let mut allowed = true;
                if let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) {
                    allowed = false;
                    for rule in rules {
                        let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("allow");
                        if rule.get("features").is_some() {
                            continue;
                        }
                        if let Some(os) = rule.get("os") {
                            let os_name = os.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            if action == "allow" && os_name == "windows" {
                                allowed = true;
                            }
                            if action == "disallow" && os_name == "windows" {
                                allowed = false;
                            }
                        } else if action == "allow" {
                            allowed = true;
                        }
                    }
                }
                if allowed {
                    if let Some(values) = obj.get("value") {
                        if let Some(arr) = values.as_array() {
                            for v in arr {
                                if let Some(s) = v.as_str() {
                                    args.push(s.replace("${library_directory", &libs_str)
                                        .replace("${classpath_separator}", sep)
                                        .replace("${version_classpath_separator}", sep));
                                }
                            }
                        } else if let Some(s) = values.as_str() {
                            args.push(s.replace("${library_directory}", &libs_str)
                                .replace("${classpath_separator}", sep)
                                .replace("${version_classpath_separator}", sep));
                        }
                    }
                }
            }
        }
    }

    // Main class from JSON
    let main_class = version_json.get("mainClass")
        .and_then(|v| v.as_str())
        .unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher");
    args.push(main_class.to_string());

    // Add game args from version JSON
    if let Some(game_args) = version_json.get("arguments").and_then(|a| a.get("game")).and_then(|a| a.as_array()) {
        for arg in game_args {
            if let Some(s) = arg.as_str() {
                let substituted = s
                    .replace("${auth_player_name}", &nickname)
                    .replace("${auth_session}", "0")
                    .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
                    .replace("${auth_access_token}", "0")
                    .replace("${clientid}", "")
                    .replace("${auth_xuid}", "")
                    .replace("${version_name}", version_name)
                    .replace("${game_directory}", &mc_dir.to_string_lossy())
                    .replace("${assets_root}", &mc_dir.join("assets").to_string_lossy())
                    .replace("${assets_index_name}", "5")
                    .replace("${user_type}", "offline")
                    .replace("${user_properties}", "{}")
                    .replace("${version_type}", "Forge")
                    .replace("${resolution_width}", &settings.window_width)
                    .replace("${resolution_height}", &settings.window_height);
                args.push(substituted);
            } else if let Some(obj) = arg.as_object() {
                let mut allowed = true;
                if let Some(rules) = obj.get("rules").and_then(|r| r.as_array()) {
                    allowed = false;
                    for rule in rules {
                        let action = rule.get("action").and_then(|v| v.as_str()).unwrap_or("allow");
                        if rule.get("features").is_some() {
                            continue;
                        }
                        if action == "allow" {
                            allowed = true;
                        }
                    }
                }
                if allowed {
                    if let Some(values) = obj.get("value") {
                        if let Some(arr) = values.as_array() {
                            for v in arr {
                                if let Some(s) = v.as_str() {
                                    args.push(s.to_string());
                                }
                            }
                        } else if let Some(s) = values.as_str() {
                            args.push(s.to_string());
                        }
                    }
                }
            }
        }
    }

    // Override: width/height/fullscreen
    if settings.fullscreen {
        args.push("--fullscreen".to_string());
    } else {
        // Remove any existing --width/--height from JSON args
        args.retain(|a| a != "--width" && a != "--height");
        args.push("--width".to_string());
        args.push(settings.window_width.clone());
        args.push("--height".to_string());
        args.push(settings.window_height.clone());
    }

    // Log the full command to a debug file
    let debug_path = version_dir.join("debug_args.txt");
    let mut debug_content = format!("java {}\n", args.join(" "));
    debug_content.push_str(&format!("\nClasspath entries: {}\n", classpath.len()));
    debug_content.push_str(&format!("Args count: {}\n", args.len()));
    let _ = fs::write(&debug_path, &debug_content);

    // Create log file
    let log_path = version_dir.join("launcher.log");
    let log_file = fs::File::create(&log_path).map_err(|e| format!("Failed to create log: {}", e))?;

    // Launch the game
    let mut child = std::process::Command::new(&java_exe)
        .args(&args)
        .current_dir(&version_dir)
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file)
        .spawn()
        .map_err(|e| format!("Failed to launch game: {}", e))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(true)
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
fn window_is_maximized(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        return Ok(window.is_maximized().unwrap_or(false));
    }
    Ok(false)
}

#[tauri::command]
async fn select_game_folder(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app
        .dialog()
        .file()
        .set_title("Select Game Folder")
        .blocking_pick_folder();

    match folder {
        Some(path) => Ok(path.to_string()),
        None => Ok(String::new()),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
