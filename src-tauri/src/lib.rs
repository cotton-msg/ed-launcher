use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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

#[tauri::command]
async fn check_server_status() -> Result<ServerStatus, String> {
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get("https://api.mcsrvstat.us/2/grand-eden.ru")
        .send()
        .await;

    let ping_ms = start.elapsed().as_millis() as u32;

    match resp {
        Ok(r) => {
            if r.status().is_success() {
                let body: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
                let online = body["online"].as_bool().unwrap_or(false);
                let players = body["players"]["online"].as_u64().unwrap_or(0) as u32;
                let max_players = body["players"]["max"].as_u64().unwrap_or(500) as u32;
                let version = body["version"]
                    .as_str()
                    .unwrap_or("1.20.4")
                    .to_string();

                Ok(ServerStatus {
                    online,
                    players,
                    max_players,
                    version,
                    ping_ms,
                    tps: 20.0,
                })
            } else {
                Ok(ServerStatus {
                    online: false,
                    players: 0,
                    max_players: 500,
                    version: "1.20.4".to_string(),
                    ping_ms,
                    tps: 0.0,
                })
            }
        }
        Err(_) => Ok(ServerStatus {
            online: false,
            players: 0,
            max_players: 500,
            version: "1.20.4".to_string(),
            ping_ms,
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

const FORGE_VERSION: &str = "1.20.1-47.2.0";
const FORGE_INSTALLER_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar";

#[derive(Debug, Deserialize)]
struct ForgeInstallProfile {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "mainClass", default)]
    main_class: Option<String>,
    #[serde(default)]
    libraries: Vec<ForgeLibrary>,
    #[serde(default)]
    arguments: Option<ForgeArguments>,
    #[serde(rename = "minecraftArguments", default)]
    minecraft_arguments: Option<String>,
    #[serde(default)]
    inherits_from: Option<String>,
    #[serde(default)]
    json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgeLibrary {
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    downloads: Option<ForgeDownloads>,
}

#[derive(Debug, Deserialize)]
struct ForgeDownloads {
    artifact: Option<ForgeArtifact>,
}

#[derive(Debug, Deserialize)]
struct ForgeArtifact {
    path: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgeArguments {
    #[serde(default)]
    game: Vec<serde_json::Value>,
    #[serde(default)]
    jvm: Vec<serde_json::Value>,
}

fn extract_forge_profile(installer_path: &PathBuf) -> Result<ForgeInstallProfile, String> {
    use std::io::Read;
    
    let jar_bytes = fs::read(installer_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&*jar_bytes))
        .map_err(|e| e.to_string())?;

    let mut install_profile_str = None;
    let mut version_str = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        
        if name == "install_profile.json" || name.ends_with("/install_profile.json") {
            let mut content = String::new();
            entry.read_to_string(&mut content).map_err(|e| e.to_string())?;
            install_profile_str = Some(content);
        } else if name == "version.json" || name.ends_with("/version.json") {
            let mut content = String::new();
            entry.read_to_string(&mut content).map_err(|e| e.to_string())?;
            version_str = Some(content);
        }
    }

    let install_profile_str = install_profile_str.ok_or("install_profile.json not found in installer JAR")?;
    let mut profile: ForgeInstallProfile = serde_json::from_str(&install_profile_str)
        .map_err(|e| e.to_string())?;

    // If profile is incomplete, try version.json
    if profile.main_class.is_none() || profile.libraries.is_empty() {
        if let Some(version_content) = version_str {
            if let Ok(version_profile) = serde_json::from_str::<ForgeInstallProfile>(&version_content) {
                if profile.main_class.is_none() {
                    profile.main_class = version_profile.main_class;
                }
                if profile.libraries.is_empty() {
                    profile.libraries = version_profile.libraries;
                }
                if profile.arguments.is_none() {
                    profile.arguments = version_profile.arguments;
                }
            }
        }
    }

    Ok(profile)
}
const JAVA_URL: &str = "https://api.adoptium.net/v3/binary/latest/17/ga/windows/x64/jdk/hotspot/normal/eclipse";

fn find_java() -> Option<String> {
    let paths = [
        "java.exe",
        "java",
        "C:\\Program Files\\Eclipse Adoptium\\jdk-17.0.19.10-hotspot\\bin\\java.exe",
        "C:\\Program Files\\Java\\jdk-17\\bin\\java.exe",
        "C:\\Program Files\\Java\\jdk-21\\bin\\java.exe",
    ];

    for path in &paths {
        if std::process::Command::new(path).arg("-version").output().is_ok() {
            return Some(path.to_string());
        }
    }

    if let Ok(output) = std::process::Command::new("where").arg("java").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("java")
                .trim()
                .to_string();
            if std::process::Command::new(&path).arg("-version").output().is_ok() {
                return Some(path);
            }
        }
    }

    None
}

async fn download_java(app: &tauri::AppHandle) -> Result<String, String> {
    use futures_util::StreamExt;

    let java_dir = get_app_data_dir(app).join("java");
    fs::create_dir_all(&java_dir).map_err(|e| e.to_string())?;

    // Check if Java was already downloaded previously
    let known_java = java_dir.join("jdk-17.0.19+10-hotspot").join("bin").join("java.exe");
    if known_java.exists() {
        return Ok(known_java.to_string_lossy().to_string());
    }

    if let Ok(entries) = fs::read_dir(&java_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let candidate = path.join("bin\\java.exe");
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }
    }

    let zip_path = java_dir.join("java.zip");

    let progress = DownloadProgress {
        file: "Downloading Java 17...".to_string(),
        progress: 0.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    };
    let _ = app.emit("download-progress", &progress);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(JAVA_URL).send().await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to download Java: HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(200_000_000) as f64;
    let total_mb = total_size / 1_048_576.0;
    let mut downloaded: f64 = 0.0;
    let mut stream = response.bytes_stream();

    let mut file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as f64;

        let progress = DownloadProgress {
            file: "Java 17".to_string(),
            progress: (downloaded / total_size) * 100.0,
            downloaded_mb: downloaded / 1_048_576.0,
            total_mb,
            speed_mb_s: 0.0,
        };
        let _ = app.emit("download-progress", &progress);
    }

    let progress = DownloadProgress {
        file: "Extracting Java...".to_string(),
        progress: 100.0,
        downloaded_mb: total_mb,
        total_mb,
        speed_mb_s: 0.0,
    };
    let _ = app.emit("download-progress", &progress);

    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                zip_path.to_string_lossy(),
                java_dir.to_string_lossy()
            ),
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("Failed to extract Java".to_string());
    }

    fs::remove_file(&zip_path).ok();

    let java_exe = java_dir.join("jdk-17.0.19+10-hotspot").join("bin").join("java.exe");
    if java_exe.exists() {
        return Ok(java_exe.to_string_lossy().to_string());
    }

    if let Ok(entries) = fs::read_dir(&java_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let candidate = path.join("bin\\java.exe");
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }
    }

    Err("Java extracted but java.exe not found".to_string())
}

fn check_rules(rules: &serde_json::Value) -> bool {
    let Some(rules) = rules.as_array() else { return true };
    let mut allowed = true;
    for rule in rules {
        let action = rule["action"].as_str().unwrap_or("allow");
        let mut applies = true;
        if let Some(os) = rule.get("os") {
            if let Some(name) = os["name"].as_str() {
                applies = match name {
                    "windows" => cfg!(windows),
                    "linux" => cfg!(target_os = "linux"),
                    "osx" => cfg!(target_os = "macos"),
                    _ => false,
                };
            }
            if let Some(arch) = os["arch"].as_str() {
                let is_x64 = cfg!(target_arch = "x86_64");
                applies = applies
                    && match arch {
                        "x86" => !is_x64,
                        "x64" => is_x64,
                        _ => false,
                    };
            }
        }
        if let Some(features) = rule.get("features") {
            // Launcher does not support custom feature toggles; ignore feature-only rules.
            if features["has_custom_resolution"].as_bool().unwrap_or(false) {
                applies = false;
            }
        }
        if applies {
            allowed = action == "allow";
        }
    }
    allowed
}

fn asset_index_name_from_json(version_json: &serde_json::Value) -> String {
    version_json["assetIndex"]["id"]
        .as_str()
        .unwrap_or("1.20")
        .to_string()
}

async fn download_with_progress(
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
        .user_agent("MinecraftLauncher/1.0")
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

async fn download_libraries(
    app: &tauri::AppHandle,
    libraries: &serde_json::Value,
    minecraft_dir: &PathBuf,
    natives_dir: &PathBuf,
    classpath_jars: &mut Vec<String>,
    _client: &reqwest::Client,
) -> Result<(), String> {
    let Some(libs) = libraries.as_array() else { return Ok(()); };

    let log_path = minecraft_dir.join("libraries_download.log");
    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();

    for lib in libs {
        if let Some(rules) = lib.get("rules") {
            if !check_rules(rules) {
                continue;
            }
        }

        let name = lib["name"].as_str().unwrap_or("");
        let is_native = name.contains(":natives-windows");

        if let Some(f) = &mut log_file {
            let _ = writeln!(f, "Processing: {} (native: {})", name, is_native);
        }

        // Check if this is a native library (by name containing natives-windows)
        if is_native {
            // Native library - download and extract
            if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
                if let (Some(url), Some(path)) = (artifact["url"].as_str(), artifact["path"].as_str()) {
                    let native_jar = minecraft_dir.join("libraries").join(path);
                    if !native_jar.exists() {
                        download_with_progress(app, &format!("native {}", name), url, &native_jar).await.ok();
                    }
                    if native_jar.exists() {
                        extract_native_jar(&native_jar, natives_dir)?;
                    }
                    // Also add to classpath
                    classpath_jars.push(native_jar.to_string_lossy().to_string());
                }
            }
            continue;
        }

        // Regular artifact library
        if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
            if let Some(path) = artifact["path"].as_str() {
                let jar_path = minecraft_dir.join("libraries").join(path);
                if !jar_path.exists() {
                    if let Some(url) = artifact["url"].as_str() {
                        download_with_progress(app, &format!("library {}", name), url, &jar_path).await.ok();
                    }
                }
                if jar_path.exists() {
                    classpath_jars.push(jar_path.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(())
}

fn extract_native_jar(jar_path: &PathBuf, natives_dir: &PathBuf) -> Result<(), String> {
    use std::io::Read;

    fs::create_dir_all(natives_dir).map_err(|e| e.to_string())?;

    let file = fs::File::open(jar_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        if name.ends_with('/') || name.starts_with("META-INF/") {
            continue;
        }

        if name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib") {
            let out_path = natives_dir.join(&name);
            // Create parent directory if needed
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&out_path).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            out_file.write_all(&buf).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn download_game(
    app: tauri::AppHandle,
    game_path: String,
) -> Result<bool, String> {
    let minecraft_dir = dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft");
    fs::create_dir_all(&minecraft_dir).ok();

    let debug_log = minecraft_dir.join("download_debug.log");
    let mut debug_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log)
        .ok();

    macro_rules! debug_log {
        ($($arg:tt)*) => {
            if let Some(f) = &mut debug_file {
                let _ = writeln!(f, "{}", format!($($arg)*));
            }
        };
    }

    debug_log!("=== Download started ===");

    let game_dir = if game_path.is_empty() {
        get_app_data_dir(&app).join("game")
    } else {
        PathBuf::from(&game_path)
    };
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    // Ensure Java is available
    debug_log!("Checking Java...");
    let _java_path = match find_java() {
        Some(p) => {
            debug_log!("Java found: {}", p);
            p
        }
        None => {
            debug_log!("Java not found, downloading...");
            download_java(&app).await?
        }
    };

    // Download Forge installer
    let installer_path = game_dir.join("forge-installer.jar");
    if !installer_path.exists() {
        debug_log!("Downloading Forge installer...");
        download_with_progress(
            &app,
            "Forge installer",
            FORGE_INSTALLER_URL,
            &installer_path,
        )
        .await?;
        debug_log!("Forge installer downloaded");
    } else {
        debug_log!("Forge installer already exists");
    }

    // Pre-download vanilla client and libraries so first launch is faster
    let minecraft_dir = dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft");
    fs::create_dir_all(&minecraft_dir).map_err(|e| e.to_string())?;

    debug_log!("Ensuring vanilla client...");
    let (version_json, _version_dir) = ensure_vanilla_client(&app, &minecraft_dir).await?;
    debug_log!("Vanilla client ready");

    debug_log!("Downloading vanilla libraries...");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("MinecraftLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let mut _cp = Vec::new();
    let natives_dir = minecraft_dir.join("natives");
    fs::create_dir_all(&natives_dir).ok();
    download_libraries(
        &app,
        &version_json["libraries"],
        &minecraft_dir,
        &natives_dir,
        &mut _cp,
        &client,
    )
    .await?;
    debug_log!("Vanilla libraries downloaded");

    let progress = DownloadProgress {
        file: "Ready to launch".to_string(),
        progress: 100.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    };
    let _ = app.emit("download-progress", &progress);

    debug_log!("=== Download completed ===");
    Ok(true)
}

async fn ensure_vanilla_client(
    app: &tauri::AppHandle,
    minecraft_dir: &PathBuf,
) -> Result<(serde_json::Value, PathBuf), String> {
    use futures_util::StreamExt;

    let version_dir = minecraft_dir.join("versions").join("1.20.1");
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;
    let vanilla_jar = version_dir.join("1.20.1.jar");
    let version_json_path = version_dir.join("1.20.1.json");

    if vanilla_jar.exists() && version_json_path.exists() {
        let version_json_str = fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;
        let version_json = serde_json::from_str(&version_json_str).map_err(|e| e.to_string())?;
        return Ok((version_json, version_dir));
    }

    let progress = DownloadProgress {
        file: "Downloading Minecraft 1.20.1...".to_string(),
        progress: 0.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    };
    let _ = app.emit("download-progress", &progress);

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("MinecraftLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let manifest: serde_json::Value = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let version_url = manifest["versions"]
        .as_array()
        .and_then(|v| v.iter().find(|v| v["id"].as_str() == Some("1.20.1")))
        .and_then(|v| v["url"].as_str())
        .ok_or("Minecraft 1.20.1 not found in manifest")?;

    let version_json: serde_json::Value = client
        .get(version_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let client_download_url = version_json["downloads"]["client"]["url"]
        .as_str()
        .ok_or("No client download URL")?;

    let resp = client
        .get(client_download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let total_size = resp.content_length().unwrap_or(23_000_000) as f64;
    let mut downloaded: f64 = 0.0;
    let mut stream = resp.bytes_stream();
    let mut file = fs::File::create(&vanilla_jar).map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as f64;
        let progress = DownloadProgress {
            file: "Minecraft 1.20.1".to_string(),
            progress: (downloaded / total_size) * 100.0,
            downloaded_mb: downloaded / 1_048_576.0,
            total_mb: total_size / 1_048_576.0,
            speed_mb_s: 0.0,
        };
        let _ = app.emit("download-progress", &progress);
    }

    let version_json_str = serde_json::to_string_pretty(&version_json).unwrap();
    fs::write(&version_json_path, version_json_str).map_err(|e| e.to_string())?;

    Ok((version_json, version_dir))
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    nickname: String,
    settings: LauncherSettings,
) -> Result<bool, String> {
    let minecraft_dir = dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft");
    fs::create_dir_all(&minecraft_dir).map_err(|e| e.to_string())?;

    let debug_log = minecraft_dir.join("launch_debug.log");
    let mut debug_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&debug_log)
        .ok();

    macro_rules! debug_log {
        ($($arg:tt)*) => {
            if let Some(f) = &mut debug_file {
                let _ = writeln!(f, "{}", format!($($arg)*));
            }
        };
    }

    debug_log!("=== Launch started ===");

    let ram_mb = (settings.ram_gb * 1024.0) as u32;

    let java_path = match find_java() {
        Some(p) => {
            debug_log!("Java found: {}", p);
            p
        }
        None => {
            debug_log!("Java not found, downloading...");
            download_java(&app).await?
        }
    };
    debug_log!("Java path: {}", java_path);

    // Download vanilla client
    debug_log!("Ensuring vanilla client...");
    let (version_json, _version_dir) = ensure_vanilla_client(&app, &minecraft_dir).await?;
    debug_log!("Vanilla client ready");

    // Download vanilla libraries
    {
        debug_log!("Downloading vanilla libraries...");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(120))
            .user_agent("MinecraftLauncher/1.0")
            .build()
            .map_err(|e| e.to_string())?;
        let mut _cp = Vec::new();
        let natives_dir = minecraft_dir.join("natives");
        fs::create_dir_all(&natives_dir).ok();
        download_libraries(&app, &version_json["libraries"], &minecraft_dir, &natives_dir, &mut _cp, &client)
            .await?;
        debug_log!("Vanilla libraries downloaded");
    }

    // Extract Forge profile from installer JAR
    let game_dir = if settings.game_path.is_empty() {
        get_app_data_dir(&app).join("game")
    } else {
        PathBuf::from(&settings.game_path)
    };
    let installer_path = game_dir.join("forge-installer.jar");
    if !installer_path.exists() {
        return Err("Forge installer not found".to_string());
    }

    debug_log!("Extracting Forge profile from installer...");
    let forge_profile = extract_forge_profile(&installer_path)?;
    debug_log!("Forge profile extracted");

    let main_class = forge_profile.main_class
        .ok_or("Forge profile has no mainClass")?;
    debug_log!("Main class: {}", main_class);

    // Download Forge libraries
    debug_log!("Downloading Forge libraries...");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("MinecraftLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let asset_index_name = "1.20";

    // Download assets
    debug_log!("Downloading assets...");
    let assets_dir = minecraft_dir.join("assets");
    let indexes_dir = assets_dir.join("indexes");
    fs::create_dir_all(&indexes_dir).map_err(|e| e.to_string())?;
    let index_path = indexes_dir.join(format!("{}.json", asset_index_name));
    
    if !index_path.exists() {
        let client = reqwest::Client::new();
        let index_url = "https://launchermeta.mojang.com/v1/packages/296a58f96d1e4ad0d7e0b1b3e3f6e8d0c5b2a1f0/1.20.json";
        let index_resp = client.get(index_url).send().await.map_err(|e| e.to_string())?;
        let index_content = index_resp.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&index_path, &index_content).map_err(|e| e.to_string())?;
    }

    let index_content = fs::read_to_string(&index_path).unwrap_or_default();
    let asset_index: serde_json::Value = serde_json::from_str(&index_content).unwrap_or(serde_json::json!({}));

    if let Some(objects) = asset_index["objects"].as_object() {
        let objects_dir = assets_dir.join("objects");
        let total = objects.len();
        let mut downloaded_count = 0;

        for (_name, info) in objects {
            let hash = info["hash"].as_str().unwrap_or("");
            if hash.is_empty() {
                downloaded_count += 1;
                continue;
            }

            let hash_prefix = &hash[..2];
            let asset_path = objects_dir.join(hash_prefix).join(hash);

            if asset_path.exists() {
                downloaded_count += 1;
                continue;
            }

            fs::create_dir_all(objects_dir.join(hash_prefix)).ok();

            let client = reqwest::Client::new();
            let asset_url = format!(
                "https://resources.download.minecraft.net/{}/{}",
                hash_prefix, hash
            );

            if let Ok(resp) = client.get(&asset_url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        fs::write(&asset_path, &bytes).ok();
                    }
                }
            }

            downloaded_count += 1;
            if downloaded_count % 100 == 0 || downloaded_count == total {
                let progress = DownloadProgress {
                    file: format!("Downloading assets ({}/{})", downloaded_count, total),
                    progress: (downloaded_count as f64 / total as f64) * 100.0,
                    downloaded_mb: downloaded_count as f64,
                    total_mb: total as f64,
                    speed_mb_s: 0.0,
                };
                let _ = app.emit("download-progress", &progress);
            }
        }
    }
    debug_log!("Assets download completed");

    let mut classpath_jars = Vec::new();
    let natives_dir = minecraft_dir.join("natives");
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    for lib in &forge_profile.libraries {
        let lib_path = if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                if let Some(path) = &artifact.path {
                    minecraft_dir.join("libraries").join(path)
                } else {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        };

        if !lib_path.exists() {
            let url = if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    artifact.url.clone()
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(url) = url {
                download_with_progress(&app, &format!("library {}", lib.name), &url, &lib_path).await.ok();
            }
        }

        if lib_path.exists() {
            classpath_jars.push(lib_path.to_string_lossy().to_string());
        }
    }
    debug_log!("Forge libraries downloaded: {} jars", classpath_jars.len());

    // Add vanilla jar to classpath
    let vanilla_jar = minecraft_dir
        .join("versions")
        .join("1.20.1")
        .join("1.20.1.jar");
    if vanilla_jar.exists() {
        classpath_jars.push(vanilla_jar.to_string_lossy().to_string());
        debug_log!("Vanilla jar added to classpath");
    }

    // Extract module path from Forge JVM args to know which jars to exclude from classpath
    let mut module_path_jars: Vec<String> = Vec::new();
    if let Some(forge_args) = &forge_profile.arguments {
        let jvm_args: Vec<String> = forge_args.jvm.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let mut i = 0;
        while i < jvm_args.len() {
            let s = &jvm_args[i];
            // Handle "-p,${path}" (comma-separated)
            if s.starts_with("-p,") {
                let path_str = s.trim_start_matches("-p,");
                let resolved = path_str.replace("${library_directory}", &minecraft_dir.join("libraries").to_string_lossy());
                module_path_jars = resolved.split(';').map(String::from).collect();
            }
            // Handle "-p" as separate arg followed by the path
            else if s == "-p" && i + 1 < jvm_args.len() {
                let path_str = &jvm_args[i + 1];
                let resolved = path_str.replace("${library_directory}", &minecraft_dir.join("libraries").to_string_lossy());
                module_path_jars = resolved.split(';').map(String::from).collect();
                i += 1;
            }
            i += 1;
        }
    }

    // Extract filenames from module path for dedup comparison
    let mp_filenames: Vec<String> = module_path_jars.iter()
        .filter_map(|e| std::path::Path::new(e).file_name()?.to_str().map(String::from))
        .collect();

    debug_log!("Module path has {} jars, excluding from classpath", mp_filenames.len());

    // Remove conflicting/duplicate libraries from classpath
    classpath_jars.retain(|jar| {
        let fname = std::path::Path::new(jar)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Skip ForgeAutoRenamingTool - conflicts with asm-commons
        if fname.contains("ForgeAutoRenamingTool") {
            return false;
        }

        // Skip any jar already on module path (avoids jopt-simple, ASM version conflicts)
        if mp_filenames.iter().any(|mp| mp == fname) {
            return false;
        }

        // Skip older ASM versions (9.2, 9.6) - 9.7 is on module path
        if fname.starts_with("asm") && fname.contains(".jar") {
            if fname.contains("asm-9.") && !fname.contains("asm-9.7") {
                return false;
            }
        }

        true
    });

    // Deduplicate classpath
    let mut seen = std::collections::HashSet::new();
    classpath_jars.retain(|x| seen.insert(x.clone()));

    let classpath = classpath_jars.join(if cfg!(windows) { ";" } else { ":" });
    debug_log!("Classpath: {} entries", classpath_jars.len());

    let asset_index_name = "1.20";

    // Build JVM arguments
    let mut args = vec![
        format!("-Xmx{}m", ram_mb),
        format!("-Xms{}m", ram_mb / 2),
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=50".to_string(),
        "-XX:+UnlockExperimentalVMOptions".to_string(),
        "-cp".to_string(),
        classpath.clone(),
    ];

    // Add Forge JVM arguments
    if let Some(forge_args) = &forge_profile.arguments {
        for arg in &forge_args.jvm {
            if let Some(s) = arg.as_str() {
                let resolved = s
                    .replace("${natives_directory}", &natives_dir.to_string_lossy())
                    .replace("${launcher_name}", "GrandEden")
                    .replace("${launcher_version}", "1.0.0")
                    .replace("${classpath}", &classpath)
                    .replace("${library_directory}", &minecraft_dir.join("libraries").to_string_lossy())
                    .replace("${classpath_separator}", if cfg!(windows) { ";" } else { ":" })
                    .replace("${version_name}", FORGE_VERSION);
                args.push(resolved);
            }
        }
    }

    if settings.vsync {
        args.push("-Dorg.lwjgl.opengl.Display.sync=true".to_string());
    }

    args.push(main_class);

    // Add Forge game arguments
    if let Some(forge_args) = &forge_profile.arguments {
        for arg in &forge_args.game {
            if let Some(s) = arg.as_str() {
                let resolved = s
                    .replace("${auth_player_name}", &nickname)
                    .replace("${version_name}", FORGE_VERSION)
                    .replace("${game_directory}", &minecraft_dir.to_string_lossy())
                    .replace("${assets_root}", &minecraft_dir.join("assets").to_string_lossy())
                    .replace("${assets_index_name}", asset_index_name)
                    .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
                    .replace("${auth_access_token}", "0")
                    .replace("${user_type}", "msa")
                    .replace("${version_type}", "release")
                    .replace("${user_properties}", "{}")
                    .replace("${auth_xuid}", "0")
                    .replace("${clientid}", "");
                args.push(resolved);
            }
        }
    }

    // Add vanilla game arguments
    args.extend_from_slice(&[
        "--username".to_string(),
        nickname.clone(),
        "--version".to_string(),
        FORGE_VERSION.to_string(),
        "--gameDir".to_string(),
        minecraft_dir.to_string_lossy().to_string(),
        "--assetsDir".to_string(),
        minecraft_dir.join("assets").to_string_lossy().to_string(),
        "--assetIndex".to_string(),
        asset_index_name.to_string(),
        "--uuid".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
        "--accessToken".to_string(),
        "0".to_string(),
        "--userType".to_string(),
        "msa".to_string(),
    ]);

    if settings.fullscreen {
        args.push("--fullscreen".to_string());
    }

    debug_log!("Total args: {}", args.len());

    // Write launch command to log and launch
    let log_path = minecraft_dir.join("launcher.log");
    let mut log_file = fs::File::create(&log_path).map_err(|e| format!("Failed to create log: {}", e))?;

    {
        let mut log_writer = std::io::BufWriter::new(&mut log_file);
        writeln!(
            log_writer,
            "Launching Minecraft with Java: {}\nArgs: {:?}\n",
            java_path, args
        )
        .ok();
        log_writer.flush().ok();
    }

    debug_log!("Launching Minecraft...");
    debug_log!("Java: {}", java_path);
    debug_log!("Args count: {}", args.len());

    let mut child = std::process::Command::new(&java_path)
        .args(&args)
        .current_dir(&minecraft_dir)
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file)
        .spawn()
        .map_err(|e| format!("Failed to launch Minecraft: {}", e))?;

    debug_log!("Minecraft process spawned: {}", child.id());

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(true)
}

fn resolve_game_arg(
    s: &str,
    nickname: &str,
    minecraft_dir: &PathBuf,
    asset_index_name: &str,
    width: &str,
    height: &str,
    fullscreen: bool,
) -> String {
    s.replace("${auth_player_name}", nickname)
        .replace("${version_name}", FORGE_VERSION)
        .replace("${game_directory}", &minecraft_dir.to_string_lossy())
        .replace("${assets_root}", &minecraft_dir.join("assets").to_string_lossy())
        .replace("${game_assets}", &minecraft_dir.join("assets").to_string_lossy())
        .replace("${assets_index_name}", asset_index_name)
        .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
        .replace("${auth_access_token}", "0")
        .replace("${user_type}", "msa")
        .replace("${version_type}", "release")
        .replace("${user_properties}", "{}")
        .replace("${resolution_width}", width)
        .replace("${resolution_height}", height)
        .replace("${fullscreen}", &fullscreen.to_string())
        .replace("${clientid}", "00000000-0000-0000-0000-000000000000")
        .replace("${auth_xuid}", "0")
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
