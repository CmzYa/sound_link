#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod devices;
mod router;

use auto_launch::AutoLaunchBuilder;
use devices::{AudioDeviceManager, Device, DeviceManager};
use router::{
    AudioRouter, RouterConfig, RouterDevice, RouterStatus, ValidationResult, VirtualDeviceStatus,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewWindow,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    advanced_material: bool,
    #[serde(default)]
    auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SavedRouterConfig {
    devices: Vec<RouterDevice>,
}

struct AppState {
    config: Mutex<AppConfig>,
    audio_manager: Mutex<AudioDeviceManager>,
    cached_data: Mutex<Option<InitialData>>,
    router: Mutex<AudioRouter>,
}

fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("sound-link");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("config.json");
    path
}

fn get_router_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("sound-link");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("router_config.json");
    path
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))
}

fn load_saved_router_config() -> SavedRouterConfig {
    let path = get_router_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    SavedRouterConfig::default()
}

fn save_router_config_file(config: &SavedRouterConfig) -> Result<(), String> {
    let path = get_router_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize router config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write router config: {}", e))
}

#[tauri::command]
fn get_audio_devices(state: tauri::State<AppState>) -> Vec<Device> {
    let manager = state.audio_manager.lock().unwrap();
    manager.get_devices()
}

#[tauri::command]
fn set_default_device(device_id: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &format!("Set-AudioDevice -Id '{}' -Default", device_id),
    ]);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set default device: {}", stderr));
    }

    Ok(())
}

#[derive(Serialize, Clone)]
struct InitialData {
    devices: Vec<Device>,
    config: AppConfig,
    timestamp: u64,
    virtual_device: VirtualDeviceStatus,
    default_device_id: Option<String>,
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[tauri::command]
fn get_initial_data(state: tauri::State<AppState>) -> InitialData {
    let devices = {
        let manager = state.audio_manager.lock().unwrap();
        let devices = manager.get_devices();
        let default_device_id = manager.get_default_device_id();
        (devices, default_device_id)
    };

    let config = state.config.lock().unwrap().clone();

    let virtual_device = {
        let router = state.router.lock().unwrap();
        router.get_virtual_device_status()
    };

    InitialData {
        devices: devices.0,
        config,
        timestamp: get_current_timestamp(),
        virtual_device,
        default_device_id: devices.1,
    }
}

#[tauri::command]
fn get_cached_data(state: tauri::State<AppState>) -> Option<InitialData> {
    state.cached_data.lock().unwrap().clone()
}

#[tauri::command]
fn refresh_and_cache(state: tauri::State<AppState>) -> InitialData {
    let devices = {
        let manager = state.audio_manager.lock().unwrap();
        let devices = manager.get_devices();
        let default_device_id = manager.get_default_device_id();
        (devices, default_device_id)
    };

    let config = state.config.lock().unwrap().clone();

    let virtual_device = {
        let router = state.router.lock().unwrap();
        router.get_virtual_device_status()
    };

    let data = InitialData {
        devices: devices.0,
        config,
        timestamp: get_current_timestamp(),
        virtual_device,
        default_device_id: devices.1,
    };

    *state.cached_data.lock().unwrap() = Some(data.clone());
    data
}

#[tauri::command]
fn hide_window(window: WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_config(
    advanced_material: Option<bool>,
    auto_start: Option<bool>,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    if let Some(material) = advanced_material {
        config.advanced_material = material;
    }
    if let Some(start) = auto_start {
        if config.auto_start != start {
            let exe_path =
                std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;

            let auto = AutoLaunchBuilder::new()
                .set_app_name("Sound Link")
                .set_app_path(&exe_path.to_string_lossy())
                .build()
                .map_err(|e| format!("Failed to create auto launch: {}", e))?;

            if start {
                auto.enable()
                    .map_err(|e| format!("Failed to enable auto start: {}", e))?;
            } else {
                auto.disable()
                    .map_err(|e| format!("Failed to disable auto start: {}", e))?;
            }
            config.auto_start = start;
        }
    }
    save_config(&config)
}

#[tauri::command]
#[cfg(target_os = "windows")]
fn get_system_accent_color() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent")
        .ok()?;

    let accent: u32 = key.get_value("AccentColorMenu").ok()?;

    let _a = ((accent >> 24) & 0xFF) as u8;
    let b = ((accent >> 16) & 0xFF) as u8;
    let g = ((accent >> 8) & 0xFF) as u8;
    let r = (accent & 0xFF) as u8;

    Some(format!("#{:02X}{:02X}{:02X}", r, g, b))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
fn get_system_accent_color() -> Option<String> {
    None
}

#[tauri::command]
#[cfg(target_os = "windows")]
fn get_system_theme() -> Option<bool> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .ok()?;

    let use_light_theme: u32 = key.get_value("SystemUsesLightTheme").ok()?;

    Some(use_light_theme == 0)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
fn get_system_theme() -> Option<bool> {
    None
}

// ========== 音频路由相关命令 ==========

#[tauri::command]
fn start_routing(device_ids: Vec<String>, state: tauri::State<AppState>) -> Result<(), String> {
    let mut router = state.router.lock().unwrap();
    router.start(device_ids)
}

#[tauri::command]
fn stop_routing(state: tauri::State<AppState>) -> Result<(), String> {
    let mut router = state.router.lock().unwrap();
    router.stop();
    Ok(())
}

#[tauri::command]
fn get_router_status(state: tauri::State<AppState>) -> RouterStatus {
    let router = state.router.lock().unwrap();
    router.get_status()
}

#[tauri::command]
fn set_router_device_volume(
    device_id: String,
    volume: f32,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut router = state.router.lock().unwrap();
    router.set_device_volume(&device_id, volume);
    Ok(())
}

#[tauri::command]
fn set_router_device_delay(
    device_id: String,
    delay_ms: u32,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut router = state.router.lock().unwrap();
    router.set_device_delay(&device_id, delay_ms);
    Ok(())
}

#[tauri::command]
fn validate_routing_targets(
    device_ids: Vec<String>,
    state: tauri::State<AppState>,
) -> ValidationResult {
    let router = state.router.lock().unwrap();
    router.validate_targets(&device_ids)
}

#[tauri::command]
fn update_router_config(config: RouterConfig, state: tauri::State<AppState>) -> Result<(), String> {
    let mut router = state.router.lock().unwrap();
    router.update_config(config);
    Ok(())
}

#[tauri::command]
fn get_router_config(state: tauri::State<AppState>) -> RouterConfig {
    let router = state.router.lock().unwrap();
    router.get_config().clone()
}

#[tauri::command]
fn get_virtual_device_status(state: tauri::State<AppState>) -> VirtualDeviceStatus {
    let router = state.router.lock().unwrap();
    router.get_virtual_device_status()
}

#[tauri::command]
fn refresh_virtual_device(state: tauri::State<AppState>) -> VirtualDeviceStatus {
    let mut router = state.router.lock().unwrap();
    router.refresh_virtual_device()
}

#[tauri::command]
fn get_saved_router_config() -> SavedRouterConfig {
    load_saved_router_config()
}

#[tauri::command]
fn get_auto_start_status(state: tauri::State<AppState>) -> bool {
    let config = state.config.lock().unwrap();

    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return config.auto_start,
    };

    if let Ok(auto) = AutoLaunchBuilder::new()
        .set_app_name("Sound Link")
        .set_app_path(&exe_path.to_string_lossy())
        .build()
    {
        let is_enabled = auto.is_enabled().unwrap_or(false);
        if is_enabled != config.auto_start {
            drop(config);
            let mut config = state.config.lock().unwrap();
            config.auto_start = is_enabled;
            let _ = save_config(&config);
            return is_enabled;
        }
        return config.auto_start;
    }
    config.auto_start
}

#[tauri::command]
fn save_router_config(config: SavedRouterConfig) -> Result<(), String> {
    save_router_config_file(&config)
}

fn show_window(window: &WebviewWindow) {
    if let Some(tray) = window.app_handle().tray_by_id("main") {
        if let Ok(Some(rect)) = tray.rect() {
            let window_width = 300;
            let window_height = 280;
            let margin = 210;

            let tray_pos: tauri::PhysicalPosition<i32> = rect.position.to_physical(1.0);
            let tray_size: tauri::PhysicalSize<i32> = rect.size.to_physical(1.0);

            let tray_x = tray_pos.x;
            let tray_y = tray_pos.y;
            let tray_h = tray_size.height;

            let mut x = tray_x;
            let mut y = tray_y + tray_h - 5;

            if let Some(monitor) = window.current_monitor().ok().flatten() {
                let work_area = monitor.work_area();
                let work_x = work_area.position.x;
                let work_y = work_area.position.y;
                let work_right = work_x + work_area.size.width as i32;
                let work_bottom = work_y + work_area.size.height as i32;

                if y + window_height > work_bottom {
                    y = tray_y - window_height - margin;
                }

                if x + window_width > work_right {
                    x = work_right - window_width - margin;
                }
                if x < work_x {
                    x = work_x + margin;
                }

                if y < work_y {
                    y = work_y + margin;
                }
                if y + window_height > work_bottom {
                    y = work_bottom - window_height - margin;
                }
            }

            let _ =
                window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
        }
    }

    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("window-shown", ());
}

fn show_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("show-settings", ());

        if let Some(tray) = app.tray_by_id("main") {
            if let Ok(Some(rect)) = tray.rect() {
                let window_width = 300;
                let window_height = 280;
                let margin = 210;

                let tray_pos: tauri::PhysicalPosition<i32> = rect.position.to_physical(1.0);
                let tray_size: tauri::PhysicalSize<i32> = rect.size.to_physical(1.0);

                let tray_x = tray_pos.x;
                let tray_y = tray_pos.y;
                let tray_h = tray_size.height;

                let mut x = tray_x;
                let mut y = tray_y + tray_h - 5;

                if let Some(monitor) = window.current_monitor().ok().flatten() {
                    let work_area = monitor.work_area();
                    let work_x = work_area.position.x;
                    let work_y = work_area.position.y;
                    let work_right = work_x + work_area.size.width as i32;
                    let work_bottom = work_y + work_area.size.height as i32;

                    if y + window_height > work_bottom {
                        y = tray_y - window_height - margin;
                    }

                    if x + window_width > work_right {
                        x = work_right - window_width - margin;
                    }
                    if x < work_x {
                        x = work_x + margin;
                    }

                    if y < work_y {
                        y = work_y + margin;
                    }
                    if y + window_height > work_bottom {
                        y = work_bottom - window_height - margin;
                    }
                }

                let _ = window
                    .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
            }
        }

        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit])?;

    let is_dark = get_system_theme().unwrap_or(false);
    let icon = create_tray_icon(is_dark);

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    show_window(&window);
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 从嵌入的 PNG 文件加载托盘图标
fn create_tray_icon(is_dark: bool) -> Image<'static> {
    // 深色主题用白色图标，浅色主题用黑色图标
    if is_dark {
        load_icon_from_bytes(include_bytes!("../icons/tray-white.png"))
    } else {
        load_icon_from_bytes(include_bytes!("../icons/tray-black.png"))
    }
}

fn load_icon_from_bytes(bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(bytes).expect("Failed to decode tray icon");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Image::new_owned(rgba.into_raw(), width, height)
}

fn main() {
    let config = load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            config: Mutex::new(config),
            audio_manager: Mutex::new(AudioDeviceManager::new()),
            cached_data: Mutex::new(None),
            router: Mutex::new(AudioRouter::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_initial_data,
            get_cached_data,
            refresh_and_cache,
            get_audio_devices,
            set_default_device,
            hide_window,
            get_config,
            set_config,
            get_system_accent_color,
            get_system_theme,
            start_routing,
            stop_routing,
            get_router_status,
            set_router_device_volume,
            set_router_device_delay,
            validate_routing_targets,
            update_router_config,
            get_router_config,
            get_virtual_device_status,
            refresh_virtual_device,
            get_saved_router_config,
            save_router_config,
            get_auto_start_status,
        ])
        .setup(|app| {
            setup_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    let _ = window.set_shadow(true);
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(mut router) = state.router.lock() {
                        if router.is_running() {
                            router.stop();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
