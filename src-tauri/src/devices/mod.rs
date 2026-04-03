use serde::{Deserialize, Serialize};

pub mod audio;

/// 音频设备
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,             // 设备ID
    pub name: String,           // 设备名称
    #[serde(rename = "type")]
    pub device_type: String,    // 设备类型 (speakers/headphones/hdmi/bluetooth)
    pub category: DeviceCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceCategory {
    Audio,
}

/// 设备管理器 trait
pub trait DeviceManager: Send + Sync {
    fn get_devices(&self) -> Vec<Device>;
    fn get_default_device_id(&self) -> Option<String>;
}

pub use audio::AudioDeviceManager;
