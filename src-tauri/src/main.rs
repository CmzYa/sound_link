#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod devices;

use devices::{Device, DeviceManager, AudioDeviceManager};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow, Emitter,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    default_device_id: Option<String>,
    #[serde(default)]
    advanced_material: bool,
}

struct AppState {
    config: Mutex<AppConfig>,
    audio_manager: Mutex<AudioDeviceManager>,
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
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config: {}", e))
}

#[tauri::command]
fn get_audio_devices(state: tauri::State<AppState>) -> Vec<Device> {
    let manager = state.audio_manager.lock().unwrap();
    manager.get_devices()
}

#[tauri::command]
fn get_default_device(state: tauri::State<AppState>) -> Option<String> {
    let manager = state.audio_manager.lock().unwrap();
    manager.get_default()
}

#[tauri::command]
fn set_default_device(device_id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let manager = state.audio_manager.lock().unwrap();
    manager.set_default(&device_id)
}

#[derive(Serialize)]
struct InitialData {
    devices: Vec<Device>,
    default_device_id: Option<String>,
    config: AppConfig,
}

#[tauri::command]
fn get_initial_data(state: tauri::State<AppState>) -> InitialData {
    let manager = state.audio_manager.lock().unwrap();
    let devices = manager.get_devices();
    let default_device_id = manager.get_default();
    let config = state.config.lock().unwrap().clone();
    
    InitialData {
        devices,
        default_device_id,
        config,
    }
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
    device_id: Option<String>,
    advanced_material: Option<bool>,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    if let Some(id) = device_id {
        config.default_device_id = if id.is_empty() { None } else { Some(id) };
    }
    if let Some(material) = advanced_material {
        config.advanced_material = material;
    }
    save_config(&config)
}

#[tauri::command]
#[cfg(target_os = "windows")]
fn get_system_accent_color() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::*;
    
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
    use winreg::RegKey;
    use winreg::enums::*;
    
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

fn show_window(window: &WebviewWindow) {
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        Ok(false) => {
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
                    
                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
                }
            }
            
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("refresh-devices", ());
        }
        Err(_) => {}
    }
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
                
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
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
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "settings" => show_settings_window(app),
                "quit" => app.exit(0),
                _ => {}
            }
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

/// 动态生成托盘图标（跑道形状）
fn create_tray_icon(is_dark: bool) -> Image<'static> {
    const SIZE: u32 = 32;
    let mut pixels: Vec<u8> = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    
    // 根据主题选择颜色：深色主题用白色图标，浅色主题用黑色图标
    let color = if is_dark { [255u8, 255u8, 255u8, 230u8] } else { [30u8, 30u8, 30u8, 230u8] };
    let transparent = [0u8, 0u8, 0u8, 0u8];
    
    let center_x = SIZE as f32 / 2.0;
    let center_y = SIZE as f32 / 2.0;
    let outer_radius = 10.0;  // 半径
    let track_width = 30.0;  // 跑道宽度
    let line_width = 3.0;    // 线条粗细
    
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32;
            let py = y as f32;
            
            // 计算到中心线的距离
            let half_width = track_width / 2.0;
            let left_center_x = center_x - half_width + outer_radius;
            let right_center_x = center_x + half_width - outer_radius;
            
            // 判断是否在跑道范围内
            let in_track = if px < left_center_x {
                // 左侧半圆
                let dx = px - left_center_x;
                let dy = py - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                dist <= outer_radius && dist >= outer_radius - line_width
            } else if px > right_center_x {
                // 右侧半圆
                let dx = px - right_center_x;
                let dy = py - center_y;
                let dist = (dx * dx + dy * dy).sqrt();
                dist <= outer_radius && dist >= outer_radius - line_width
            } else {
                // 中间直线部分
                let dist_to_center = (py - center_y).abs();
                let in_top_line = dist_to_center >= outer_radius - line_width && dist_to_center <= outer_radius;
                let in_bottom_line = dist_to_center >= outer_radius - line_width && dist_to_center <= outer_radius;
                in_top_line || in_bottom_line
            };
            
            let pixel = if in_track { color } else { transparent };
            pixels.extend_from_slice(&pixel);
        }
    }
    
    Image::new_owned(pixels, SIZE, SIZE)
}

fn main() {
    let config = load_config();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            config: Mutex::new(config),
            audio_manager: Mutex::new(AudioDeviceManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_initial_data,
            get_audio_devices,
            get_default_device,
            set_default_device,
            hide_window,
            get_config,
            set_config,
            get_system_accent_color,
            get_system_theme,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
