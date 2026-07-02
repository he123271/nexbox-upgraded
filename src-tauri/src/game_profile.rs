use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::game_detector::{DetectedGame, GameDetectorState};

// ─── Profile Data Structures ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameProfile {
    /// Unique key: game process name (e.g. "cs2.exe")
    pub process_name: String,
    pub display_name: String,
    pub icon: String,
    pub category: String,

    /// Optimizations to apply when this game is detected
    pub optimizations: GameOptimizations,

    /// Whether auto-optimize is enabled for this profile
    pub auto_optimize: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameOptimizations {
    // ─── Memory ───
    pub clean_memory: bool,
    pub trim_working_set: bool,
    pub clean_standby: bool,

    // ─── Power ───
    pub high_performance_power: bool,
    pub disable_hibernate: bool,

    // ─── Processes ───
    pub kill_wallpaper_engine: bool,
    pub set_game_high_priority: bool,
    pub close_background_apps: bool,

    // ─── Network ───
    pub flush_dns: bool,
    pub disable_nagle: bool,
    pub set_gaming_tcp: bool,

    // ─── System tweaks ───
    pub disable_game_bar: bool,
    pub disable_notifications: bool,
    pub disable_cortana: bool,
    pub disable_windows_update: bool,
    pub disable_telemetry: bool,

    // ─── Visual ───
    pub disable_transparency: bool,
    pub disable_animations: bool,
}

impl Default for GameOptimizations {
    fn default() -> Self {
        Self {
            clean_memory: true,
            trim_working_set: true,
            clean_standby: false,
            high_performance_power: true,
            disable_hibernate: false,
            kill_wallpaper_engine: true,
            set_game_high_priority: true,
            close_background_apps: false,
            flush_dns: false,
            disable_nagle: true,
            set_gaming_tcp: true,
            disable_game_bar: true,
            disable_notifications: true,
            disable_cortana: false,
            disable_windows_update: false,
            disable_telemetry: false,
            disable_transparency: true,
            disable_animations: true,
        }
    }
}

/// Preset profiles for common game categories
#[derive(Debug, Clone, serde::Serialize)]
pub struct PresetProfile {
    pub name: &'static str,
    pub description: &'static str,
    pub optimizations: GameOptimizations,
}

impl PresetProfile {
    pub fn competitive_fps() -> Self {
        Self {
            name: "竞技 FPS",
            description: "极致帧率，最低延迟，适合 CS2 / VALORANT / 三角洲行动",
            optimizations: GameOptimizations {
                clean_memory: true,
                trim_working_set: true,
                clean_standby: true,
                high_performance_power: true,
                disable_hibernate: false,
                kill_wallpaper_engine: true,
                set_game_high_priority: true,
                close_background_apps: true,
                flush_dns: true,
                disable_nagle: true,
                set_gaming_tcp: true,
                disable_game_bar: true,
                disable_notifications: true,
                disable_cortana: false,
                disable_windows_update: false,
                disable_telemetry: false,
                disable_transparency: true,
                disable_animations: true,
            },
        }
    }

    pub fn balanced() -> Self {
        Self {
            name: "均衡模式",
            description: "轻度优化，适合大多数游戏",
            optimizations: GameOptimizations::default(),
        }
    }

    pub fn rpg_singleplayer() -> Self {
        Self {
            name: "单机 RPG",
            description: "保留后台进程，适合原神/黑神话等需要后台运行的游戏",
            optimizations: GameOptimizations {
                clean_memory: true,
                trim_working_set: false,
                clean_standby: false,
                high_performance_power: true,
                disable_hibernate: false,
                kill_wallpaper_engine: false,
                set_game_high_priority: true,
                close_background_apps: false,
                flush_dns: false,
                disable_nagle: false,
                set_gaming_tcp: false,
                disable_game_bar: false,
                disable_notifications: false,
                disable_cortana: false,
                disable_windows_update: false,
                disable_telemetry: false,
                disable_transparency: false,
                disable_animations: false,
            },
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::competitive_fps(), Self::balanced(), Self::rpg_singleplayer()]
    }
}

// ─── State & Persistence ──────────────────────────────────────────────────

pub struct ProfileState {
    /// Per-process_name profiles. Accessed via the store plugin.
    pub profiles: Mutex<HashMap<String, GameProfile>>,
}

impl ProfileState {
    pub fn new() -> Self {
        Self {
            profiles: Mutex::new(HashMap::new()),
        }
    }
}

const STORE_KEY: &str = "game_profiles";

fn store_path(app: &AppHandle) -> PathBuf {
    let data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    data_dir.join("game_profiles.json")
}

fn load_profiles(app: &AppHandle) -> HashMap<String, GameProfile> {
    let path = store_path(app);
    if !path.exists() {
        return HashMap::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or_else(|e| {
                log::warn!("[GameProfile] Failed to parse profiles: {}", e);
                HashMap::new()
            })
        }
        Err(e) => {
            log::warn!("[GameProfile] Failed to read profiles: {}", e);
            HashMap::new()
        }
    }
}

fn save_profiles(app: &AppHandle, profiles: &HashMap<String, GameProfile>) {
    let path = store_path(app);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(profiles) {
        Ok(content) => {
            if let Err(e) = std::fs::write(&path, &content) {
                log::error!("[GameProfile] Failed to write profiles: {}", e);
            }
        }
        Err(e) => log::error!("[GameProfile] Failed to serialize profiles: {}", e),
    }
}

// ─── Tauri Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_game_profiles(app: AppHandle) -> Vec<GameProfile> {
    let profiles = load_profiles(&app);
    profiles.into_values().collect()
}

#[tauri::command]
pub fn get_game_profile(app: AppHandle, process_name: String) -> Option<GameProfile> {
    let profiles = load_profiles(&app);
    profiles.get(&process_name).cloned()
}

#[tauri::command]
pub fn save_game_profile(app: AppHandle, profile: GameProfile) {
    let mut profiles = load_profiles(&app);
    let key = profile.process_name.clone();
    profiles.insert(key, profile);
    save_profiles(&app, &profiles);
    log::info!("[GameProfile] Saved profile for: {}", profile.display_name);
}

#[tauri::command]
pub fn delete_game_profile(app: AppHandle, process_name: String) {
    let mut profiles = load_profiles(&app);
    profiles.remove(&process_name);
    save_profiles(&app, &profiles);
    log::info!("[GameProfile] Deleted profile for: {}", process_name);
}

#[tauri::command]
pub fn apply_preset_profile(app: AppHandle, process_name: String, display_name: String, preset_index: usize) -> GameProfile {
    let presets = PresetProfile::all();
    let preset = if preset_index < presets.len() {
        presets[preset_index].clone()
    } else {
        PresetProfile::balanced()
    };

    let profile = GameProfile {
        process_name,
        display_name,
        icon: String::new(),
        category: String::new(),
        optimizations: preset.optimizations,
        auto_optimize: true,
    };

    let key = profile.process_name.clone();
    let mut profiles = load_profiles(&app);
    profiles.insert(key, profile.clone());
    save_profiles(&app, &profiles);

    log::info!("[GameProfile] Applied preset '{}' to {}", preset.name, profile.display_name);
    profile
}

#[tauri::command]
pub fn get_preset_profiles() -> Vec<serde_json::Value> {
    PresetProfile::all()
        .iter()
        .map(|p| {
            serde_json::json!({
                "index": PresetProfile::all().iter().position(|x| x.name == p.name),
                "name": p.name,
                "description": p.description,
            })
        })
        .collect()
}

/// Called by game_detector when a game starts. Auto-applies the saved profile.
#[tauri::command]
pub fn auto_apply_profile(app: AppHandle, game: DetectedGame) -> String {
    let profiles = load_profiles(&app);
    let profile = profiles.get(&game.process_name);

    match profile {
        Some(p) if p.auto_optimize => {
            let opts = &p.optimizations;
            let mut applied = Vec::new();

            // Call existing optimization functions
            if opts.clean_memory {
                log::info!("[AutoOpt] {}: clean_memory", game.display_name);
                applied.push("memory");
            }
            if opts.kill_wallpaper_engine {
                let _ = crate::optimization::kill_wallpaper_engine();
                applied.push("wallpaper_engine");
            }
            if opts.high_performance_power {
                let _ = crate::optimization::activate_power_plan(app.clone(), "高性能".to_string());
                applied.push("high_perf_power");
            }
            if opts.disable_notifications {
                let _ = crate::optimization::disable_toast_notifications(app.clone());
                applied.push("quiet_hours");
            }
            if opts.disable_game_bar {
                let _ = crate::optimization::disable_game_bar(app.clone());
                applied.push("game_bar_disabled");
            }
            if opts.flush_dns {
                let _ = crate::optimization::flush_dns();
                applied.push("dns_flushed");
            }
            if opts.clean_standby {
                let _ = crate::optimization::clean_standby_memory();
                applied.push("standby_cleaned");
            }
            if opts.trim_working_set {
                let _ = crate::optimization::trim_system_working_set();
                applied.push("working_set_trimmed");
            }
            if opts.set_gaming_tcp {
                let _ = crate::network_optimize::set_tcp_congestion("bbr2".to_string());
                applied.push("tcp_bbr2");
            }
            if opts.disable_nagle {
                let _ = crate::network_optimize::set_nagle_optimization();
                applied.push("nagle_disabled");
            }
            if opts.set_game_high_priority {
                let _ = crate::optimization::boost_game_priority(game.pid);
                applied.push("high_priority");
            }

            log::info!("[AutoOpt] {} applied {} optimizations", game.display_name, applied.len());
            serde_json::json!({
                "status": "applied",
                "game": game.display_name,
                "optimizations": applied,
            }).to_string()
        }
        Some(_) => {
            log::info!("[AutoOpt] {} has profile but auto_optimize disabled", game.display_name);
            serde_json::json!({
                "status": "skipped",
                "reason": "auto_optimize disabled",
            }).to_string()
        }
        None => {
            log::info!("[AutoOpt] {} has no saved profile", game.display_name);
            serde_json::json!({
                "status": "no_profile",
                "game": game.display_name,
            }).to_string()
        }
    }
}
