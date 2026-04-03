mod delay_buffer;
mod router;
mod volume_sync;

pub use router::AudioRouter;

use serde::{Deserialize, Serialize};

/// 路由设备配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterDevice {
    pub id: String,         // 设备ID
    pub name: String,       // 设备名称
    pub volume: f32,        // 音量百分比 (0.0-1.0)
    pub delay_ms: u32,      // 延迟毫秒
    pub enabled: bool,      // 是否启用
}

/// 路由配置
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct RouterConfig {
    pub devices: Vec<RouterDevice>,
}

/// 路由状态
#[derive(Clone, Serialize)]
pub struct RouterStatus {
    pub is_running: bool,                       // 是否运行中
    pub source_device_id: Option<String>,       // 源设备ID (VB-Cable)
    pub source_device_name: Option<String>,     // 源设备名称
    pub target_devices: Vec<RouterDevice>,      // 目标设备列表
    pub vb_cable_id: Option<String>,            // VB-Cable ID
    pub original_default_device_id: Option<String>, // 原默认设备ID
}

/// 虚拟设备状态
#[derive(Clone, Serialize)]
pub struct VirtualDeviceStatus {
    pub is_installed: bool,         // 是否已安装
    pub device_id: Option<String>,  // 设备ID
    pub device_name: Option<String>,// 设备名称
}

/// 验证结果
#[derive(Clone, Serialize)]
pub struct ValidationResult {
    pub has_conflicts: bool,            // 是否有冲突
    pub conflict_devices: Vec<String>,  // 冲突设备列表
    pub warning: String,                // 警告信息
}
