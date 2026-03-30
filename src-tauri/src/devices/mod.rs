use serde::{Deserialize, Serialize};

pub mod audio;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub category: DeviceCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceCategory {
    Audio,
}

pub trait DeviceManager: Send + Sync {
    fn get_devices(&self) -> Vec<Device>;
    fn get_default_device_id(&self) -> Option<String>;
}

pub use audio::AudioDeviceManager;
