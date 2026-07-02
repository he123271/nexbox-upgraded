use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS, PROCESSENTRY32W,
};

// ─── Game Database ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GameCategory {
    FPS,
    MOBA,
    RPG,
    Strategy,
    Simulation,
    Other,
}

impl GameCategory {
    fn as_str(&self) -> &'static str {
        match self {
            GameCategory::FPS => "FPS",
            GameCategory::MOBA => "MOBA",
            GameCategory::RPG => "RPG",
            GameCategory::Strategy => "Strategy",
            GameCategory::Simulation => "Simulation",
            GameCategory::Other => "Other",
        }
    }
}

struct GameEntry {
    process_name: &'static str,
    display_name: &'static str,
    category: GameCategory,
    icon: &'static str, // emoji as placeholder
    aliases: &'static [&'static str], // alternative process names
}

// Sorted by popularity; Steam/launchers at the bottom so games take priority
const GAME_DATABASE: &[GameEntry] = &[
    // === FPS / Battle Royale ===
    GameEntry { process_name: "cs2.exe",        display_name: "Counter-Strike 2",          category: GameCategory::FPS,        icon: "🎯", aliases: &[] },
    GameEntry { process_name: "r5apex.exe",     display_name: "Apex Legends",              category: GameCategory::FPS,        icon: "🎯", aliases: &[] },
    GameEntry { process_name: "valorant.exe",   display_name: "VALORANT",                  category: GameCategory::FPS,        icon: "🎯", aliases: &["Valorant.exe", "VALORANT-Win64-Shipping.exe"] },
    GameEntry { process_name: "DeltaForceClient-Win64-Shipping.exe", display_name: "三角洲行动", category: GameCategory::FPS, icon: "⚔️", aliases: &["DeltaForce.exe"] },
    GameEntry { process_name: "TslGame.exe",    display_name: "PUBG: BATTLEGROUNDS",       category: GameCategory::FPS,        icon: "🎯", aliases: &["PUBG.exe"] },
    GameEntry { process_name: "FortniteClient-Win64-Shipping.exe", display_name: "Fortnite", category: GameCategory::FPS, icon: "🎯", aliases: &[] },
    GameEntry { process_name: "overwatch.exe",  display_name: "Overwatch 2",               category: GameCategory::FPS,        icon: "🎯", aliases: &["Overwatch.exe"] },
    GameEntry { process_name: "RainbowSix.exe", display_name: "Rainbow Six Siege",         category: GameCategory::FPS,        icon: "🎯", aliases: &["RainbowSix_BE.exe"] },
    GameEntry { process_name: "EscapeFromTarkov.exe", display_name: "Escape from Tarkov", category: GameCategory::FPS, icon: "🎯", aliases: &[] },
    GameEntry { process_name: "cod.exe",        display_name: "Call of Duty",              category: GameCategory::FPS,        icon: "🎯", aliases: &["COD.exe", "BlackOpsColdWar.exe", "ModernWarfare.exe"] },
    GameEntry { process_name: "HuntGame.exe",   display_name: "Hunt: Showdown",            category: GameCategory::FPS,        icon: "🎯", aliases: &[] },
    GameEntry { process_name: "ReadyOrNot.exe", display_name: "Ready or Not",              category: GameCategory::FPS,        icon: "🎯", aliases: &[] },
    GameEntry { process_name: "Bodycam.exe",    display_name: "Bodycam",                   category: GameCategory::FPS,        icon: "🎯", aliases: &[] },

    // === MOBA / Action ===
    GameEntry { process_name: "League of Legends.exe", display_name: "League of Legends", category: GameCategory::MOBA, icon: "⚔️", aliases: &["LeagueClient.exe"] },
    GameEntry { process_name: "DOTA2.exe",      display_name: "Dota 2",                    category: GameCategory::MOBA,       icon: "⚔️", aliases: &[] },
    GameEntry { process_name: "MobileLegends.exe", display_name: "Mobile Legends",        category: GameCategory::MOBA,       icon: "⚔️", aliases: &[] },

    // === RPG / Open-world ===
    GameEntry { process_name: "GenshinImpact.exe", display_name: "原神",                   category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "StarRail.exe",   display_name: "Honkai: Star Rail",         category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "ZenlessZoneZero.exe", display_name: "Zenless Zone Zero",    category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "Cyberpunk2077.exe", display_name: "Cyberpunk 2077",         category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "eldenring.exe",  display_name: "Elden Ring",                category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "BlackMythWukong.exe", display_name: "Black Myth: Wukong",   category: GameCategory::RPG,        icon: "🗺️", aliases: &["Wukong.exe", "b1-Win64-Shipping.exe"] },
    GameEntry { process_name: "MonsterHunterWorld.exe", display_name: "Monster Hunter: World", category: GameCategory::RPG, icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "MonsterHunterRise.exe", display_name: "Monster Hunter: Rise", category: GameCategory::RPG, icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "Ghost_of_Tsushima.exe", display_name: "Ghost of Tsushima",  category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "GTA5.exe",        display_name: "Grand Theft Auto V",       category: GameCategory::RPG,        icon: "🗺️", aliases: &["GTAV.exe"] },
    GameEntry { process_name: "RDR2.exe",        display_name: "Red Dead Redemption 2",    category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "Wow.exe",         display_name: "World of Warcraft",        category: GameCategory::RPG,        icon: "🗺️", aliases: &["WowClassic.exe"] },
    GameEntry { process_name: "FFXIV.exe",       display_name: "Final Fantasy XIV",        category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "ffxiv_dx11.exe",  display_name: "Final Fantasy XIV",        category: GameCategory::RPG,        icon: "🗺️", aliases: &[] },
    GameEntry { process_name: "Sekiro.exe",      display_name: "Sekiro: Shadows Die Twice", category: GameCategory::RPG,       icon: "🗺️", aliases: &[] },

    // === Strategy / Simulation ===
    GameEntry { process_name: "CivilizationVI.exe", display_name: "Sid Meier's Civilization VI", category: GameCategory::Strategy, icon: "🏰", aliases: &[] },
    GameEntry { process_name: "Age_of_Empires_IV.exe", display_name: "Age of Empires IV", category: GameCategory::Strategy, icon: "🏰", aliases: &[] },
    GameEntry { process_name: "AoE2DE_s.exe",   display_name: "Age of Empires II: DE",     category: GameCategory::Strategy,   icon: "🏰", aliases: &[] },
    GameEntry { process_name: "StarCraftII.exe", display_name: "StarCraft II",              category: GameCategory::Strategy,   icon: "🏰", aliases: &[] },
    GameEntry { process_name: "Factorio.exe",    display_name: "Factorio",                  category: GameCategory::Simulation, icon: "🏭", aliases: &[] },
    GameEntry { process_name: "Satisfactory.exe",display_name: "Satisfactory",              category: GameCategory::Simulation, icon: "🏭", aliases: &[] },

    // === Sandbox / Creative ===
    GameEntry { process_name: "Minecraft.exe",  display_name: "Minecraft Java",             category: GameCategory::Simulation, icon: "⛏️", aliases: &["javaw.exe"] },
    GameEntry { process_name: "Minecraft.Windows.exe", display_name: "Minecraft Bedrock",  category: GameCategory::Simulation, icon: "⛏️", aliases: &[] },

    // === Platform launchers (last — only match if nothing else) ===
    GameEntry { process_name: "steam.exe",      display_name: "Steam",                      category: GameCategory::Other,      icon: "🟦", aliases: &["Steam.exe"] },
    GameEntry { process_name: "EADesktop.exe",  display_name: "EA App",                     category: GameCategory::Other,      icon: "🟦", aliases: &[] },
    GameEntry { process_name: "EpicGamesLauncher.exe", display_name: "Epic Games Launcher", category: GameCategory::Other,      icon: "🟦", aliases: &[] },
    GameEntry { process_name: "Battle.net.exe", display_name: "Battle.net",                 category: GameCategory::Other,      icon: "🟦", aliases: &[] },
    GameEntry { process_name: "UbisoftConnect.exe", display_name: "Ubisoft Connect",        category: GameCategory::Other,      icon: "🟦", aliases: &[] },
    GameEntry { process_name: "XboxPcApp.exe",  display_name: "Xbox App",                   category: GameCategory::Other,      icon: "🟦", aliases: &[] },
];

// Build a HashMap for O(1) lookup: process_name_lower -> &GameEntry
fn game_map() -> &'static HashMap<&'static str, &'static GameEntry> {
    static GAME_MAP: OnceLock<HashMap<&'static str, &'static GameEntry>> = OnceLock::new();
    GAME_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        for entry in GAME_DATABASE {
            // Leak the lowercase string to get a &'static str
            let key = entry.process_name.to_lowercase();
            m.insert(key.leak(), entry);
            for alias in entry.aliases {
                let alias_key = alias.to_lowercase();
                m.insert(alias_key.leak(), entry);
            }
        }
        m
    })
}

// ─── State ────────────────────────────────────────────────────────────────

pub struct GameDetectorState {
    pub current_game: Mutex<Option<DetectedGame>>,
    pub previous_pid: Mutex<u32>,
    pub detector_enabled: AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectedGame {
    pub process_name: String,
    pub display_name: String,
    pub category: String,
    pub icon: String,
    pub pid: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GameDetectorEvent {
    pub game: Option<DetectedGame>,
    pub action: String, // "started" | "stopped"
}

impl Default for GameDetectorState {
    fn default() -> Self {
        Self {
            current_game: Mutex::new(None),
            previous_pid: Mutex::new(0),
            detector_enabled: AtomicBool::new(true),
        }
    }
}

// ─── Windows API helpers ──────────────────────────────────────────────────

fn get_foreground_process_name() -> Option<(String, u32)> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_invalid() || hwnd.0.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        // Try QueryFullProcessImageNameW
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if handle.is_err() || handle.unwrap().is_invalid() {
            return None;
        }
        let handle = handle.unwrap();

        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(handle, 0, &mut buf, &mut size);

        let _ = CloseHandle(handle);

        if result.as_bool() && size > 0 {
            let exe_path = String::from_utf16_lossy(&buf[..size as usize]);
            // Extract file name from path
            let exe_name = std::path::Path::new(&exe_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            Some((exe_name, pid))
        } else {
            None
        }
    }
}

// Need CloseHandle
use windows::Win32::Foundation::CloseHandle;

// ─── Game lookup ──────────────────────────────────────────────────────────

fn lookup_game(process_name: &str) -> Option<&'static GameEntry> {
    let lower = process_name.to_lowercase();
    game_map().get(lower.as_str()).copied()
}

fn is_launcher(entry: &GameEntry) -> bool {
    matches!(entry.category, GameCategory::Other)
}

// ─── Polling loop ─────────────────────────────────────────────────────────

pub async fn start_detector_loop(app: AppHandle) {
    let state = app.state::<GameDetectorState>();
    let mut interval = time::interval(Duration::from_millis(1500));

    loop {
        interval.tick().await;

        if !state.detector_enabled.load(Ordering::Relaxed) {
            let mut prev = state.previous_pid.lock().unwrap();
            *prev = 0;
            continue;
        }

        let detection = get_foreground_process_name();
        let mut prev_pid = state.previous_pid.lock().unwrap();

        match detection {
            Some((exe_name, pid)) if pid != *prev_pid => {
                *prev_pid = pid;

                // Check if it's a known game
                let known = lookup_game(&exe_name);

                if let Some(entry) = known {
                    // Skip launchers
                    if is_launcher(entry) {
                        let mut game = state.current_game.lock().unwrap();
                        if game.is_some() {
                            let old = game.take();
                            log::info!("[GameDetector] Launcher detected, stopping game tracking");
                            let _ = app.emit("game-detector", GameDetectorEvent {
                                game: None,
                                action: "stopped".to_string(),
                            });
                            // Also call optimization restore
                        }
                        continue;
                    }

                    let detected = DetectedGame {
                        process_name: exe_name,
                        display_name: entry.display_name.to_string(),
                        category: entry.category.as_str().to_string(),
                        icon: entry.icon.to_string(),
                        pid,
                    };

                    log::info!("[GameDetector] Game started: {} (PID: {})", detected.display_name, pid);
                    let mut game = state.current_game.lock().unwrap();
                    game.replace(detected.clone());

                    let _ = app.emit("game-detector", GameDetectorEvent {
                        game: Some(detected.clone()),
                        action: "started".to_string(),
                    });
                } else {
                    // Unknown foreground app — if we were tracking a game, it stopped
                    let mut game = state.current_game.lock().unwrap();
                    if game.is_some() {
                        let old = game.take();
                        log::info!("[GameDetector] Game stopped: {:?}", old.as_ref().map(|g| &g.display_name));
                        let _ = app.emit("game-detector", GameDetectorEvent {
                            game: None,
                            action: "stopped".to_string(),
                        });
                    }
                }
            }
            None if *prev_pid != 0 => {
                // No foreground window (desktop/lockscreen)
                *prev_pid = 0;
                let mut game = state.current_game.lock().unwrap();
                if game.is_some() {
                    game.take();
                    let _ = app.emit("game-detector", GameDetectorEvent {
                        game: None,
                        action: "stopped".to_string(),
                    });
                }
            }
            _ => {} // No change
        }
    }
}

// ─── Tauri Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_current_game(state: State<'_, GameDetectorState>) -> Option<DetectedGame> {
    state.current_game.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_game_detector_enabled(state: State<'_, GameDetectorState>, enabled: bool) {
    state.detector_enabled.store(enabled, Ordering::Relaxed);
    log::info!("[GameDetector] {}", if enabled { "Enabled" } else { "Disabled" });
}

#[tauri::command]
pub fn get_game_detector_enabled(state: State<'_, GameDetectorState>) -> bool {
    state.detector_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_known_games() -> Vec<serde_json::Value> {
    GAME_DATABASE
        .iter()
        .filter(|e| !is_launcher(e))
        .map(|e| {
            serde_json::json!({
                "process_name": e.process_name,
                "display_name": e.display_name,
                "category": e.category.as_str(),
                "icon": e.icon,
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_game_categories() -> Vec<String> {
    vec![
        "FPS".into(),
        "MOBA".into(),
        "RPG".into(),
        "Strategy".into(),
        "Simulation".into(),
        "Other".into(),
    ]
}
