use super::{Device, DeviceCategory, DeviceManager};

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
use windows::{
    core::*,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
    Win32::Storage::EnhancedStorage::PKEY_Devices_FriendlyName,
    Win32::Foundation::BOOL,
};

pub struct AudioDeviceManager;

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl DeviceManager for AudioDeviceManager {
    fn get_devices(&self) -> Vec<Device> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let result = self.get_devices_inner();
            CoUninitialize();
            result
        }
    }
    
    fn get_default(&self) -> Option<String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let result = self.get_default_inner();
            CoUninitialize();
            result
        }
    }
    
    fn set_default(&self, device_id: &str) -> std::result::Result<(), String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let result = self.set_default_inner(device_id);
            CoUninitialize();
            result
        }
    }
}

#[cfg(windows)]
impl AudioDeviceManager {
    unsafe fn get_devices_inner(&self) -> Vec<Device> {
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
                if let Some(d) = self.device_to_struct(&device) {
                    devices.push(d);
                }
            }
        }
        
        devices
    }
    
    unsafe fn device_to_struct(&self, device: &IMMDevice) -> Option<Device> {
        let id = device.GetId().ok()?;
        let id_str = id.to_string().ok()?;
        
        // 尝试获取设备友好名称，如果失败则使用空字符串
        let mut name = if let Ok(store) = device.OpenPropertyStore(STGM_READ) {
            if let Ok(prop_variant) = store.GetValue(&PKEY_Devices_FriendlyName) {
                prop_variant.to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        // 如果名字为空，尝试从设备 ID 提取友好名称
        if name.is_empty() || name.trim().is_empty() {
            name = extract_name_from_id(&id_str);
        }
        
        let (device_type, clean_name) = parse_device_info(&id_str, &name);
        
        // 确保 clean_name 不为空
        let final_name = if clean_name.is_empty() || clean_name.trim().is_empty() {
            extract_name_from_id(&id_str)
        } else {
            clean_name
        };
        
        Some(Device {
            id: id_str,
            name: final_name,
            device_type,
            category: DeviceCategory::Audio,
        })
    }
    
    unsafe fn get_default_inner(&self) -> Option<String> {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        
        let id = device.GetId().ok()?;
        
        id.to_string().ok()
    }
    
    unsafe fn set_default_inner(&self, device_id: &str) -> std::result::Result<(), String> {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create enumerator: {}", e))?;
        
        let device = enumerator.GetDevice(&HSTRING::from(device_id))
            .map_err(|e| format!("Failed to get device: {}", e))?;
        
        let id = device.GetId()
            .map_err(|e| format!("Failed to get device id: {}", e))?;
        
        let policy_config = PolicyConfig::new()
            .map_err(|e| format!("Failed to create PolicyConfig: {}", e))?;
        
        policy_config.set_default_endpoint(PCWSTR(id.as_ptr()), eConsole)
            .map_err(|e| format!("Failed to set default endpoint: {}", e))?;
        
        policy_config.set_default_endpoint(PCWSTR(id.as_ptr()), eMultimedia)
            .map_err(|e| format!("Failed to set default endpoint for multimedia: {}", e))?;
        
        policy_config.set_default_endpoint(PCWSTR(id.as_ptr()), eCommunications)
            .map_err(|e| format!("Failed to set default endpoint for communications: {}", e))?;
        
        Ok(())
    }
}

#[cfg(not(windows))]
impl DeviceManager for AudioDeviceManager {
    fn get_devices(&self) -> Vec<Device> {
        Vec::new()
    }
    
    fn get_default(&self) -> Option<String> {
        None
    }
    
    fn set_default(&self, _device_id: &str) -> std::result::Result<(), String> {
        Err("Not supported on this platform".to_string())
    }
}

// 从设备 ID 提取友好名称
fn extract_name_from_id(id: &str) -> String {
    // Windows 音频设备 ID 格式示例:
    // {0.0.0.00000000}.{...}\\\\?SWD#MMDEVAPI#{0.0.0.00000000}.{...}#{eRender}#{...}
    // 尝试提取其中的有用信息
    
    // 查找设备类型关键字
    let id_lower = id.to_lowercase();
    
    if id_lower.contains("hdmi") {
        return "HDMI 音频设备".to_string();
    } else if id_lower.contains("usb") {
        return "USB 音频设备".to_string();
    } else if id_lower.contains("bluetooth") {
        return "蓝牙音频设备".to_string();
    } else if id_lower.contains("speaker") {
        return "扬声器".to_string();
    } else if id_lower.contains("headphone") {
        return "耳机".to_string();
    } else if id_lower.contains("microphone") || id_lower.contains("mic") {
        return "麦克风".to_string();
    }
    
    // 尝试从 ID 中提取 GUID 或其他标识符的最后部分
    if let Some(last_part) = id.split('#').last() {
        if !last_part.is_empty() && last_part.len() < 50 {
            return last_part.to_string();
        }
    }
    
    // 如果无法提取，返回通用名称
    "音频设备".to_string()
}

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

#[cfg(windows)]
const CLSID_PolicyConfig: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

#[cfg(windows)]
const IID_IPolicyConfig: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f29029470);

#[cfg(windows)]
#[repr(C)]
struct IPolicyConfigVtable {
    query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    get_mix_format: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut *mut WAVEFORMATEX) -> HRESULT,
    get_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut *mut WAVEFORMATEX) -> HRESULT,
    set_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut WAVEFORMATEX, *mut WAVEFORMATEX) -> HRESULT,
    get_processing_period: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut i64) -> HRESULT,
    set_processing_period: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut i64) -> HRESULT,
    get_share_mode: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut AUDCLNT_SHAREMODE) -> HRESULT,
    set_share_mode: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut AUDCLNT_SHAREMODE) -> HRESULT,
    get_stream_flags: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut u32) -> HRESULT,
    set_stream_flags: unsafe extern "system" fn(*mut c_void, PCWSTR, u32) -> HRESULT,
    set_default_endpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, ERole) -> HRESULT,
    set_endpoint_visibility: unsafe extern "system" fn(*mut c_void, PCWSTR, BOOL) -> HRESULT,
}

#[cfg(windows)]
#[repr(transparent)]
struct IPolicyConfig(*mut IPolicyConfigVtable);

#[cfg(windows)]
impl IPolicyConfig {
    unsafe fn set_default_endpoint(&self, device_id: PCWSTR, role: ERole) -> Result<()> {
        ((*self.0).set_default_endpoint)(self.0 as *mut c_void, device_id, role).ok()
    }
}

#[cfg(windows)]
struct PolicyConfig {
    inner: IPolicyConfig,
}

#[cfg(windows)]
impl PolicyConfig {
    fn new() -> Result<Self> {
        unsafe {
            let unknown: IUnknown = CoCreateInstance(&CLSID_PolicyConfig, None, CLSCTX_ALL)?;
            let mut ptr: *mut c_void = std::ptr::null_mut();
            unknown.query(&IID_IPolicyConfig, &mut ptr).ok()?;
            Ok(Self {
                inner: IPolicyConfig(ptr as *mut IPolicyConfigVtable),
            })
        }
    }
    
    unsafe fn set_default_endpoint(&self, device_id: PCWSTR, role: ERole) -> Result<()> {
        self.inner.set_default_endpoint(device_id, role)
    }
}

#[cfg(windows)]
impl Drop for PolicyConfig {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.0.is_null() {
                ((*self.inner.0).release)(self.inner.0 as *mut c_void);
            }
        }
    }
}
