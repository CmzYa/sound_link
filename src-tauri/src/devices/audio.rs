use super::{Device, DeviceCategory, DeviceManager};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 音频设备管理器 - 通过 PowerShell 管理系统音频设备
pub struct AudioDeviceManager;

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self::ensure_module_installed();
        Self
    }

    /// 确保 AudioDeviceCmdlets 模块已安装
    fn ensure_module_installed() {
        if !Self::is_module_installed() {
            Self::install_module();
        }
    }

    /// 检查 AudioDeviceCmdlets 模块是否已安装
    fn is_module_installed() -> bool {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Get-Module -ListAvailable -Name AudioDeviceCmdlets | Select-Object -First 1",
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("AudioDeviceCmdlets");
        }
        false
    }

    /// 安装 AudioDeviceCmdlets 模块
    fn install_module() {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Install-Module -Name AudioDeviceCmdlets -Force -Scope CurrentUser",
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let _ = cmd.output();
    }
}

impl DeviceManager for AudioDeviceManager {
    /// 获取所有音频播放设备列表
    fn get_devices(&self) -> Vec<Device> {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-AudioDevice -List | Where-Object { $_.Type -eq 'Playback' } | ForEach-Object { \"$($_.Id)|$($_.Name)\" }"
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output();

        let mut devices = Vec::new();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("Active code page:") {
                    continue;
                }

                // 解析格式：设备ID|设备名称
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    let device_id = parts[0].to_string();
                    let raw_name = parts[1].to_string();
                    let (device_type, clean_name) = parse_device_info(&device_id, &raw_name);

                    devices.push(Device {
                        id: device_id,
                        name: clean_name,
                        device_type,
                        category: DeviceCategory::Audio,
                    });
                }
            }
        }

        devices
    }

    /// 获取当前默认播放设备ID
    fn get_default_device_id(&self) -> Option<String> {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-AudioDevice -Playback | Select-Object -ExpandProperty Id"
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("Active code page:") {
                continue;
            }
            return Some(line.to_string());
        }

        None
    }
}

/// 解析设备信息，提取设备类型和清洁的设备名称
///
/// # Arguments
/// * `device_id` - 设备ID
/// * `raw_name` - 原始设备名称
///
/// # Returns
/// (设备类型, 清洁的设备名称)
fn parse_device_info(device_id: &str, raw_name: &str) -> (String, String) {
    let name_lower = raw_name.to_lowercase();
    let id_lower = device_id.to_lowercase();

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
        let name = extract_hardware_name(raw_name, "蓝牙");
        ("bluetooth".to_string(), name)
    } else {
        ("speakers".to_string(), raw_name.to_string())
    };

    (device_type, clean_name)
}

/// 从中文设备名称中提取硬件名称
///
/// 例如："扬声器 (Realtek Audio)" -> "Realtek Audio"
fn extract_hardware_name(raw_name: &str, prefix: &str) -> String {
    if let Some(paren_start) = raw_name.find('(') {
        if let Some(paren_end) = raw_name.rfind(')') {
            let inner = &raw_name[paren_start + 1..paren_end];
            return inner.to_string();
        }
    }
    raw_name.replace(prefix, "").trim().to_string()
}

/// 从英文设备名称中提取硬件名称
fn extract_hardware_name_english(raw_name: &str, _prefix: &str) -> String {
    if let Some(paren_start) = raw_name.find('(') {
        if let Some(paren_end) = raw_name.rfind(')') {
            let inner = &raw_name[paren_start + 1..paren_end];
            return inner.to_string();
        }
    }
    raw_name.to_string()
}
