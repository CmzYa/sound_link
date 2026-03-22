#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow, Emitter,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AudioDevice {
    id: String,
    name: String,
    #[serde(rename = "type")]
    device_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    default_device_id: Option<String>,
    #[serde(default)]
    advanced_material: bool,
}

struct AppState {
    config: Mutex<AppConfig>,
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

#[cfg(target_os = "windows")]
mod audio {
    use super::AudioDevice;
    use windows::{
        core::*,
        Win32::Media::Audio::*,
        Win32::System::Com::*,
        Win32::Devices::Properties::*,
        Win32::Storage::StructuredStorage::*,
    };

    fn parse_device_info(id: &str, raw_name: &str) -> (String, String) {
        let name_lower = raw_name.to_lowercase();
        let id_lower = id.to_lowercase();
        
        let (device_type, clean_name) = if name_lower.contains("耳机") {
            let name = extract_hardware_name(raw_name, "耳机");
            ("headphones".to_string(), name)
        } else if name_lower.contains("扬声器") {
            let name = extract_hardware_name(raw_name, "扬声器");
            ("speakers".to_string(), name)
        } else if name_lower.contains("headphone") {
            let name = extract_hardware_name_english(raw_name, "headphone");
            ("headphones".to_string(), name)
        } else if name_lower.contains("speaker") {
            let name = extract_hardware_name_english(raw_name, "speaker");
            ("speakers".to_string(), name)
        } else if name_lower.contains("hdmi") || id_lower.contains("hdmi") {
            ("hdmi".to_string(), raw_name.to_string())
        } else if name_lower.contains("bluetooth") || name_lower.contains("蓝牙") {
            ("bluetooth".to_string(), raw_name.to_string())
        } else {
            ("speakers".to_string(), raw_name.to_string())
        };
        
        (device_type, clean_name)
    }

    fn extract_hardware_name(raw_name: &str, prefix: &str) -> String {
        if let Some(paren_start) = raw_name.find('(') {
            if let Some(paren_end) = raw_name.rfind(')') {
                let inner = &raw_name[paren_start + 1..paren_end];
                return inner.to_string();
            }
        }
        raw_name.replace(prefix, "").trim().to_string()
    }

    fn extract_hardware_name_english(raw_name: &str, _prefix: &str) -> String {
        if let Some(paren_start) = raw_name.find('(') {
            if let Some(paren_end) = raw_name.rfind(')') {
                let inner = &raw_name[paren_start + 1..paren_end];
                return inner.to_string();
            }
        }
        raw_name.to_string()
    }

    unsafe fn get_device_friendly_name(device: &IMMDevice) -> String {
        let store = match device.OpenPropertyStore(STGM_READ) {
            Ok(s) => s,
            Err(_) => return String::from("Unknown Device"),
        };
        
        let mut prop = PROPVARIANT::default();
        if store.GetValue(&DEVPKEY_Device_FriendlyName, &mut prop).is_err() {
            return String::from("Unknown Device");
        }
        
        let name = prop.to_string();
        let _ = PropVariantClear(&mut prop);
        name
    }

    unsafe fn get_device_id(device: &IMMDevice) -> Result<String> {
        let id_ptr = device.GetId()?;
        Ok(id_ptr.to_string()?)
    }

    pub fn get_audio_devices() -> Vec<AudioDevice> {
        unsafe {
            let _ = CoInitialize(None);
            
            let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                Ok(e) => e,
                Err(_) => return Vec::new(),
            };
            
            let collection = match enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            
            let count = match collection.GetCount() {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            
            let mut devices = Vec::new();
            
            for i in 0..count {
                if let Ok(device) = collection.Item(i) {
                    if let Ok(id) = get_device_id(&device) {
                        let name = get_device_friendly_name(&device);
                        let (device_type, clean_name) = parse_device_info(&id, &name);
                        devices.push(AudioDevice {
                            id,
                            name: clean_name,
                            device_type,
                        });
                    }
                }
            }
            
            CoUninitialize();
            devices
        }
    }

    pub fn get_default_device() -> Option<String> {
        unsafe {
            let _ = CoInitialize(None);
            
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            let id = get_device_id(&device).ok();
            
            CoUninitialize();
            id
        }
    }

    pub fn set_default_device(device_id: &str) -> Result<()> {
        unsafe {
            let _ = CoInitialize(None);
            
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let _device = enumerator.GetDevice(&HSTRING::from(device_id))?;
            
            let policy_config_clsid = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
            
            let policy: IPolicyConfig = CoCreateInstance(&policy_config_clsid, None, CLSCTX_ALL)?;
            
            policy.SetDefaultEndpoint(&HSTRING::from(device_id), eConsole)?;
            policy.SetDefaultEndpoint(&HSTRING::from(device_id), eMultimedia)?;
            policy.SetDefaultEndpoint(&HSTRING::from(device_id), eCommunications)?;
            
            CoUninitialize();
            Ok(())
        }
    }
    
    #[allow(non_upper_case_globals)]
    const eConsole: EDataFlow = EDataFlow(0);
    #[allow(non_upper_case_globals)]
    const eMultimedia: EDataFlow = EDataFlow(1);
    #[allow(non_upper_case_globals)]
    const eCommunications: EDataFlow = EDataFlow(2);
}

windows::core::interface! {
    #[doc(hidden)]
    IPolicyConfig(IUnknown): windows::core::IUnknown_vtable {
        fn SetDefaultEndpoint(device_id: PCWSTR, role: EDataFlow) -> windows::core::HRESULT,
    }
}

#[cfg(not(target_os = "windows"))]
mod audio {
    use super::AudioDevice;

    pub fn get_audio_devices() -> Vec<AudioDevice> {
        Vec::new()
    }

    pub fn get_default_device() -> Option<String> {
        None
    }

    pub fn set_default_device(_device_id: &str) -> Result<(), String> {
        Err("Not supported on this platform".to_string())
    }
}

#[tauri::command]
fn get_audio_devices() -> Vec<AudioDevice> {
    audio::get_audio_devices()
}

#[tauri::command]
fn get_default_device() -> Option<String> {
    audio::get_default_device()
}

#[tauri::command]
fn set_default_device(device_id: String) -> Result<(), String> {
    if device_id.is_empty() {
        return Ok(());
    }
    audio::set_default_device(&device_id)
        .map_err(|e| format!("Failed to set device: {}", e))
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
    
    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
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

fn main() {
    let config = load_config();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            config: Mutex::new(config),
        })
        .invoke_handler(tauri::generate_handler![
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
