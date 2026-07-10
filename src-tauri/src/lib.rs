use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

// GitHub repository configuration  
const GITHUB_REPO_OWNER: &str = "supminer"; // Replace with your GitHub username
const GITHUB_REPO_NAME: &str = "ed-launcher"; // Replace with your repo name
const GITHUB_BRANCH: &str = "main";

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

// Get list of files in GitHub repository folder
fn get_github_tree(path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + '_>> {
    Box::pin(async move {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
            GITHUB_REPO_OWNER, GITHUB_REPO_NAME, path, GITHUB_BRANCH
        );

        let client = reqwest::Client::builder()
            .user_agent("GrandEdenLauncher/1.0")
            .build()
            .map_err(|e| e.to_string())?;

        let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get GitHub tree: HTTP {}", response.status()));
        }

        let items: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;
        let mut files = Vec::new();

        for item in items {
            if let Some(item_type) = item["type"].as_str() {
                if item_type == "file" {
                    if let Some(path) = item["path"].as_str() {
                        files.push(path.to_string());
                    }
                } else if item_type == "dir" {
                    if let Some(subpath) = item["path"].as_str() {
                        // Recursively get files from subdirectories
                        match get_github_tree(subpath).await {
                            Ok(subfiles) => files.extend(subfiles),
                            Err(_) => continue,
                        }
                    }
                }
            }
        }

        Ok(files)
    })
}

#[tauri::command]
async fn download_game(
    app: tauri::AppHandle,
    _game_path: String,
) -> Result<bool, String> {
    let game_dir = get_app_data_dir(&app).join("game");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    // Download Java from GitHub
    let java_dir = game_dir.join("java");
    if !java_dir.exists() || !java_dir.join("bin").join("java.exe").exists() {
        let _ = app.emit("download-progress", &DownloadProgress {
            file: "Downloading Java from GitHub...".to_string(),
            progress: 0.0,
            downloaded_mb: 0.0,
            total_mb: 0.0,
            speed_mb_s: 0.0,
        });

        // Get list of Java files from GitHub
        let java_files = get_github_tree("java").await?;
        let total_files = java_files.len();

        for (index, file_path) in java_files.iter().enumerate() {
            let url = format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                GITHUB_REPO_OWNER, GITHUB_REPO_NAME, GITHUB_BRANCH, file_path
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

    // Download version folder from GitHub
    let version_dir = game_dir.join("version");
    if !version_dir.exists() {
        let _ = app.emit("download-progress", &DownloadProgress {
            file: "Downloading game files from GitHub...".to_string(),
            progress: 0.0,
            downloaded_mb: 0.0,
            total_mb: 0.0,
            speed_mb_s: 0.0,
        });

        // Get list of version files from GitHub
        let version_files = get_github_tree("version").await?;
        let total_files = version_files.len();

        for (index, file_path) in version_files.iter().enumerate() {
            let url = format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                GITHUB_REPO_OWNER, GITHUB_REPO_NAME, GITHUB_BRANCH, file_path
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

    let _ = app.emit("download-progress", &DownloadProgress {
        file: "Ready to launch".to_string(),
        progress: 100.0,
        downloaded_mb: 0.0,
        total_mb: 0.0,
        speed_mb_s: 0.0,
    });

    Ok(true)
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    nickname: String,
    settings: LauncherSettings,
) -> Result<bool, String> {
    let game_dir = get_app_data_dir(&app).join("game");
    let java_exe = game_dir.join("java").join("bin").join("java.exe");
    let version_dir = game_dir.join("version");

    if !java_exe.exists() {
        return Err("Java not found. Please download game files first.".to_string());
    }

    if !version_dir.exists() {
        return Err("Game files not found. Please download game files first.".to_string());
    }

    // Build classpath from mods folder
    let mods_dir = version_dir.join("mods");
    let mut classpath = Vec::new();

    if mods_dir.exists() {
        if let Ok(entries) = fs::read_dir(&mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("jar") {
                    classpath.push(path.to_string_lossy().to_string());
                }
            }
        }
    }

    let classpath_str = classpath.join(if cfg!(windows) { ";" } else { ":" });

    // Build JVM arguments
    let ram_mb = (settings.ram_gb * 1024.0) as u32;
    let mut args = vec![
        format!("-Xmx{}m", ram_mb),
        format!("-Xms{}m", ram_mb / 2),
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=50".to_string(),
        "-Djava.library.path=".to_string() + &version_dir.to_string_lossy(),
    ];

    if !classpath_str.is_empty() {
        args.push("-cp".to_string());
        args.push(classpath_str);
    }

    // Main class for Forge
    args.push("cpw.mods.bootstraplauncher.BootstrapLauncher".to_string());

    // Game arguments
    args.extend_from_slice(&[
        "--username".to_string(),
        nickname.clone(),
        "--gameDir".to_string(),
        version_dir.to_string_lossy().to_string(),
    ]);

    if settings.fullscreen {
        args.push("--fullscreen".to_string());
    } else {
        args.extend_from_slice(&[
            "--width".to_string(),
            settings.window_width.clone(),
            "--height".to_string(),
            settings.window_height.clone(),
        ]);
    }

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
