use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;

/// 音量同步器 - 虚拟设备与目标设备之间的双向音量同步
pub struct VolumeSync {
    target_volume_controls: HashMap<String, IAudioEndpointVolume>,  // 目标设备音量控制接口
    volume_percentages: Arc<Mutex<HashMap<String, f32>>>,           // 软件音量百分比
    last_virtual_device_volume: f32,                                  // 上次虚拟设备音量
    last_target_system_volumes: HashMap<String, f32>,                // 上次目标设备音量
    last_sync_time: Instant,                                         // 上次同步时间（防抖）
}

struct TargetVolumeChange {
    device_id: String,
    new_virtual_device_volume: f32,
}

impl VolumeSync {
    const SYNC_DEBOUNCE_MS: u64 = 50;      // 防抖间隔（毫秒）
    const VOLUME_CHANGE_THRESHOLD: f32 = 0.01;  // 音量变化阈值

    pub fn new(volume_percentages: Arc<Mutex<HashMap<String, f32>>>) -> Self {
        Self {
            target_volume_controls: HashMap::new(),
            volume_percentages,
            last_virtual_device_volume: 1.0,
            last_target_system_volumes: HashMap::new(),
            last_sync_time: Instant::now(),
        }
    }

    pub fn add_target(&mut self, device_id: String, volume_control: IAudioEndpointVolume) {
        self.last_target_system_volumes.insert(device_id.clone(), 1.0);
        self.target_volume_controls.insert(device_id, volume_control);
    }

    /// 主同步方法：虚拟设备音量变化同步到目标，目标音量变化同步回虚拟设备
    pub unsafe fn sync(&mut self, virtual_device_volume_control: &IAudioEndpointVolume) {
        let now = Instant::now();
        if now.duration_since(self.last_sync_time) < Duration::from_millis(Self::SYNC_DEBOUNCE_MS) {
            return;
        }
        self.last_sync_time = now;

        // 获取虚拟设备音量和静音状态
        let is_muted = virtual_device_volume_control
            .GetMute()
            .unwrap_or(windows::Win32::Foundation::BOOL(0))
            .as_bool();

        let current_virtual_volume: f32 = if is_muted {
            0.0
        } else {
            virtual_device_volume_control.GetMasterVolumeLevelScalar().unwrap_or(1.0)
        };

        // 虚拟设备音量变化 -> 同步到所有目标
        let virtual_volume_changed = 
            (current_virtual_volume - self.last_virtual_device_volume).abs() > Self::VOLUME_CHANGE_THRESHOLD;

        if virtual_volume_changed {
            self.last_virtual_device_volume = current_virtual_volume;
            self.sync_virtual_to_all_targets(current_virtual_volume, is_muted);
        } else {
            // 目标设备音量变化 -> 反向同步
            if let Some(change) = self.detect_target_volume_change(current_virtual_volume) {
                let _ = virtual_device_volume_control.SetMasterVolumeLevelScalar(
                    change.new_virtual_device_volume,
                    std::ptr::null(),
                );
                self.last_virtual_device_volume = change.new_virtual_device_volume;
                self.sync_virtual_to_other_targets(change.new_virtual_device_volume, is_muted, &change.device_id);
            }
        }
    }

    /// 虚拟设备 -> 所有目标设备
    /// 目标音量 = 虚拟音量 * 音量百分比
    unsafe fn sync_virtual_to_all_targets(&mut self, virtual_volume: f32, is_muted: bool) {
        for (device_id, volume_control) in self.target_volume_controls.iter() {
            let volume_percentage = self
                .volume_percentages
                .lock()
                .ok()
                .and_then(|v| v.get(device_id).copied())
                .unwrap_or(1.0);

            let target_system_volume = virtual_volume * volume_percentage;
            
            let _ = volume_control.SetMasterVolumeLevelScalar(target_system_volume, std::ptr::null());
            let _ = volume_control.SetMute(is_muted, std::ptr::null());

            self.last_target_system_volumes.insert(device_id.clone(), target_system_volume);
        }
    }

    /// 检测目标设备音量变化
    unsafe fn detect_target_volume_change(&mut self, current_virtual_volume: f32) -> Option<TargetVolumeChange> {
        for (device_id, volume_control) in self.target_volume_controls.iter() {
            if let Ok(current_target_volume) = volume_control.GetMasterVolumeLevelScalar() {
                let last_target_volume = self
                    .last_target_system_volumes
                    .get(device_id)
                    .copied()
                    .unwrap_or(1.0);

                if (current_target_volume - last_target_volume).abs() > Self::VOLUME_CHANGE_THRESHOLD {
                    let volume_percentage = self
                        .volume_percentages
                        .lock()
                        .ok()
                        .and_then(|v| v.get(device_id).copied())
                        .unwrap_or(1.0);

                    if volume_percentage > Self::VOLUME_CHANGE_THRESHOLD {
                        // 反向计算：虚拟音量 = 目标音量 / 音量百分比
                        let calculated_virtual_volume = (current_target_volume / volume_percentage).clamp(0.0, 1.0);

                        if (calculated_virtual_volume - current_virtual_volume).abs() > Self::VOLUME_CHANGE_THRESHOLD {
                            return Some(TargetVolumeChange {
                                device_id: device_id.clone(),
                                new_virtual_device_volume: calculated_virtual_volume,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// 同步到除触发设备外的其他目标设备
    unsafe fn sync_virtual_to_other_targets(
        &mut self,
        virtual_volume: f32,
        is_muted: bool,
        trigger_device_id: &str,
    ) {
        for (device_id, volume_control) in self.target_volume_controls.iter() {
            if device_id == trigger_device_id {
                if let Ok(vol) = volume_control.GetMasterVolumeLevelScalar() {
                    self.last_target_system_volumes.insert(device_id.clone(), vol);
                }
                continue;
            }

            let volume_percentage = self
                .volume_percentages
                .lock()
                .ok()
                .and_then(|v| v.get(device_id).copied())
                .unwrap_or(1.0);

            let target_system_volume = virtual_volume * volume_percentage;
            let _ = volume_control.SetMasterVolumeLevelScalar(target_system_volume, std::ptr::null());
            let _ = volume_control.SetMute(is_muted, std::ptr::null());

            self.last_target_system_volumes.insert(device_id.clone(), target_system_volume);
        }
    }
}
