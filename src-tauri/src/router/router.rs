use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use scopeguard::defer;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

use super::delay_buffer::DelayBuffer;
use crate::router::{RouterConfig, RouterStatus, ValidationResult, VirtualDeviceStatus};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 音频路由器核心
pub struct AudioRouter {
    config: RouterConfig,
    source_device_id: Option<String>,
    source_device_name: Option<String>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
    delay_buffers: HashMap<String, DelayBuffer>,
    sample_rate: u32,
    channels: u32,
    vb_cable_id: Option<String>,
    original_default_device_id: Option<String>,
    // 线程安全的共享数据
    shared_volumes: Arc<Mutex<HashMap<String, f32>>>,
    shared_delays: Arc<Mutex<HashMap<String, u32>>>,
}

impl AudioRouter {
    pub fn new() -> Self {
        let vb_cable_id = Self::find_vb_cable_device();
        Self {
            config: RouterConfig::default(),
            source_device_id: None,
            source_device_name: None,
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            delay_buffers: HashMap::new(),
            sample_rate: 48000,
            channels: 2,
            vb_cable_id,
            original_default_device_id: None,
            shared_volumes: Arc::new(Mutex::new(HashMap::new())),
            shared_delays: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 查找 VB-Cable 虚拟设备（通过 PowerShell）
    fn find_vb_cable_device() -> Option<String> {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            "chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-AudioDevice -List | Where-Object { $_.Type -eq 'Playback' } | Where-Object { $_.Name -match 'CABLE Input|VB-Audio|VB-Cable|Virtual Cable' } | Select-Object -First 1 -ExpandProperty Id"
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let id = stdout.trim().to_string();
            if !id.is_empty() && !id.starts_with("Active code page:") {
                return Some(id);
            }
        }
        None
    }

    /// 获取 VB-Cable 状态
    pub fn get_virtual_device_status(&self) -> VirtualDeviceStatus {
        VirtualDeviceStatus {
            is_installed: self.vb_cable_id.is_some(),
            device_id: self.vb_cable_id.clone(),
            device_name: self
                .vb_cable_id
                .as_ref()
                .and_then(|id| Self::get_device_name_by_id(id)),
        }
    }

    /// 重新检测虚拟设备
    pub fn refresh_virtual_device(&mut self) -> VirtualDeviceStatus {
        self.vb_cable_id = Self::find_vb_cable_device();
        self.get_virtual_device_status()
    }

    /// 获取设备名称（通过 PowerShell）
    fn get_device_name_by_id(device_id: &str) -> Option<String> {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &format!("chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; (Get-AudioDevice -Id '{}').Name", device_id)
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let name = stdout.trim().to_string();
            if !name.is_empty() && !name.starts_with("Active code page:") {
                return Some(name);
            }
        }
        None
    }

    /// 获取默认输出设备ID
    pub fn get_default_output_device_id(&self) -> Result<String, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            defer! { CoUninitialize(); }

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("Failed to create enumerator: {:?}", e))?;

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| format!("Failed to get default device: {:?}", e))?;

            let id = device
                .GetId()
                .map_err(|e| format!("Failed to get device id: {:?}", e))?;

            Ok(id
                .to_string()
                .map_err(|e| format!("Failed to convert id: {:?}", e))?)
        }
    }

    /// 验证路由目标
    pub fn validate_targets(&self, device_ids: &[String]) -> ValidationResult {
        let default_id = self.get_default_output_device_id().ok();

        let conflicts: Vec<String> = device_ids
            .iter()
            .filter(|id| Some(id.as_str()) == default_id.as_deref())
            .cloned()
            .collect();

        ValidationResult {
            has_conflicts: !conflicts.is_empty(),
            conflict_devices: conflicts,
            warning: if default_id.is_some() {
                "源设备将被自动排除以避免回音".to_string()
            } else {
                String::new()
            },
        }
    }

    /// 启动路由
    pub fn start(&mut self, target_device_ids: Vec<String>) -> Result<(), String> {
        // 如果已经在运行，先停止再重启以更新目标设备
        if self.running.load(Ordering::SeqCst) {
            self.stop();
            thread::sleep(Duration::from_millis(50));
        }

        // 检查 VB-Cable 是否已安装
        let vb_cable_id = self.vb_cable_id.clone();
        if vb_cable_id.is_none() {
            return Err("未检测到 VB-Cable 虚拟设备，请先安装 VB-Cable".to_string());
        }

        let vb_cable_id = vb_cable_id.unwrap();

        // 保存当前默认设备
        let current_default = self.get_default_output_device_id()?;
        self.original_default_device_id = Some(current_default.clone());

        // 如果当前默认设备不是 VB-Cable，切换到 VB-Cable
        if current_default != vb_cable_id {
            self.set_default_device(&vb_cable_id)?;
        }

        // 设置源设备为 VB-Cable
        self.source_device_id = Some(vb_cable_id.clone());
        self.source_device_name = Self::get_device_name_by_id(&vb_cable_id);

        // 过滤目标设备（排除 VB-Cable）
        let valid_targets: Vec<String> = target_device_ids
            .into_iter()
            .filter(|id| id != &vb_cable_id)
            .collect();

        if valid_targets.is_empty() {
            // 恢复原默认设备
            if let Some(original) = &self.original_default_device_id {
                let _ = self.set_default_device(original);
            }
            return Err("没有有效的目标设备".to_string());
        }

        // 初始化延迟缓冲区
        self.delay_buffers.clear();
        for device in &self.config.devices {
            if valid_targets.contains(&device.id) && device.enabled {
                self.delay_buffers.insert(
                    device.id.clone(),
                    DelayBuffer::new(device.delay_ms, self.sample_rate, self.channels as usize),
                );
            }
        }

        // 初始化共享数据
        {
            let mut volumes = self.shared_volumes.lock().unwrap();
            let mut delays = self.shared_delays.lock().unwrap();
            volumes.clear();
            delays.clear();
            for device in &self.config.devices {
                if valid_targets.contains(&device.id) && device.enabled {
                    volumes.insert(device.id.clone(), device.volume);
                    delays.insert(device.id.clone(), device.delay_ms);
                }
            }
        }

        // 启动路由线程
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let config = self.config.clone();
        let shared_volumes = self.shared_volumes.clone();
        let shared_delays = self.shared_delays.clone();

        let handle = thread::spawn(move || unsafe {
            if let Err(e) = Self::router_loop(
                vb_cable_id,
                valid_targets,
                config,
                running,
                shared_volumes,
                shared_delays,
            ) {
                eprintln!("Router loop error: {}", e);
            }
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    /// 设置默认设备（通过 PowerShell）
    fn set_default_device(&self, device_id: &str) -> Result<(), String> {
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

    /// 停止路由
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.delay_buffers.clear();

        // 恢复原默认设备
        if let Some(original_id) = &self.original_default_device_id {
            let _ = self.set_default_device(original_id);
        }
        self.original_default_device_id = None;
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取状态
    pub fn get_status(&self) -> RouterStatus {
        RouterStatus {
            is_running: self.running.load(Ordering::SeqCst),
            source_device_id: self.source_device_id.clone(),
            source_device_name: self.source_device_name.clone(),
            target_devices: self.config.devices.clone(),
            vb_cable_id: self.vb_cable_id.clone(),
            original_default_device_id: self.original_default_device_id.clone(),
        }
    }

    /// 设置设备音量
    pub fn set_device_volume(&mut self, device_id: &str, volume: f32) {
        if let Some(device) = self.config.devices.iter_mut().find(|d| d.id == device_id) {
            device.volume = volume;
        }
        // 更新共享数据，使运行中的线程可以实时获取
        if let Ok(mut volumes) = self.shared_volumes.lock() {
            volumes.insert(device_id.to_string(), volume);
        }
    }

    /// 设置设备延迟
    pub fn set_device_delay(&mut self, device_id: &str, delay_ms: u32) {
        if let Some(buffer) = self.delay_buffers.get_mut(device_id) {
            buffer.set_delay(delay_ms, self.sample_rate);
        }
        if let Some(device) = self.config.devices.iter_mut().find(|d| d.id == device_id) {
            device.delay_ms = delay_ms;
        }
        // 更新共享数据，使运行中的线程可以实时获取
        if let Ok(mut delays) = self.shared_delays.lock() {
            delays.insert(device_id.to_string(), delay_ms);
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: RouterConfig) {
        self.config = config;
    }

    /// 获取配置
    pub fn get_config(&self) -> &RouterConfig {
        &self.config
    }

    unsafe fn router_loop(
        source_device_id: String,
        target_ids: Vec<String>,
        config: RouterConfig,
        running: Arc<AtomicBool>,
        shared_volumes: Arc<Mutex<HashMap<String, f32>>>,
        shared_delays: Arc<Mutex<HashMap<String, u32>>>,
    ) -> Result<(), String> {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        defer! { CoUninitialize(); }

        // 初始化捕获（从 VB-Cable 捕获）
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create enumerator: {:?}", e))?;

        let source_id = windows::core::HSTRING::from(source_device_id.as_str());
        let capture_device = enumerator
            .GetDevice(&source_id)
            .map_err(|e| format!("Failed to get VB-Cable device: {:?}", e))?;

        let capture_client: IAudioClient = capture_device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to activate capture client: {:?}", e))?;

        let format_ptr = capture_client
            .GetMixFormat()
            .map_err(|e| format!("Failed to get format: {:?}", e))?;

        let wave_format = &*format_ptr;
        let sample_rate = wave_format.nSamplesPerSec;
        let channels = wave_format.nChannels as u32;

        capture_client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                10_000_000,
                0,
                format_ptr,
                None,
            )
            .map_err(|e| format!("Failed to initialize capture: {:?}", e))?;

        let capture: IAudioCaptureClient = capture_client
            .GetService()
            .map_err(|e| format!("Failed to get capture client: {:?}", e))?;
        capture_client
            .Start()
            .map_err(|e| format!("Failed to start capture: {:?}", e))?;
        defer! { let _ = capture_client.Stop(); }

        // 获取源设备(VB-Cable)的系统音量控制接口
        let source_volume: IAudioEndpointVolume = capture_device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to get source volume interface: {:?}", e))?;

        // 初始化渲染客户端（使用源设备格式确保一致性）
        let mut render_clients: HashMap<String, IAudioClient> = HashMap::new();
        let mut render_outputs: HashMap<String, IAudioRenderClient> = HashMap::new();
        let mut delay_buffers: HashMap<String, DelayBuffer> = HashMap::new();
        // 目标设备的系统音量控制接口（用于音量同步）
        let mut target_volume_controls: HashMap<String, IAudioEndpointVolume> = HashMap::new();

        for device_config in &config.devices {
            if !target_ids.contains(&device_config.id) || !device_config.enabled {
                continue;
            }

            let id = windows::core::HSTRING::from(device_config.id.as_str());
            if let Ok(device) = enumerator.GetDevice(&id) {
                // 获取目标设备的系统音量控制接口
                if let Ok(endpoint_volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    target_volume_controls.insert(device_config.id.clone(), endpoint_volume);
                }

                if let Ok(audio_client) = device.Activate::<IAudioClient>(CLSCTX_ALL, None) {
                    // 使用源设备格式初始化，确保音频质量一致
                    if audio_client
                        .Initialize(
                            AUDCLNT_SHAREMODE_SHARED,
                            AUDCLNT_STREAMFLAGS_NOPERSIST,
                            10_000_000,
                            0,
                            format_ptr,
                            None,
                        )
                        .is_ok()
                    {
                        if let Ok(render_client) = audio_client.GetService::<IAudioRenderClient>() {
                            let _ = audio_client.Start();
                            render_clients.insert(device_config.id.clone(), audio_client);
                            render_outputs.insert(device_config.id.clone(), render_client);
                            delay_buffers.insert(
                                device_config.id.clone(),
                                DelayBuffer::new(
                                    device_config.delay_ms,
                                    sample_rate,
                                    channels as usize,
                                ),
                            );
                        }
                    }
                }
            }
        }

        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));

            // 实时更新延迟设置
            if let Ok(delays) = shared_delays.lock() {
                for (device_id, &delay_ms) in delays.iter() {
                    if let Some(buffer) = delay_buffers.get_mut(device_id) {
                        buffer.set_delay(delay_ms, sample_rate);
                    }
                }
            }

            // 获取源设备(VB-Cable)的系统音量和静音状态
            let is_muted = source_volume.GetMute().unwrap_or(windows::Win32::Foundation::BOOL(0)).as_bool();
            let system_volume: f32 = if is_muted {
                0.0
            } else {
                source_volume.GetMasterVolumeLevelScalar().unwrap_or(1.0)
            };

            // 音量同步：将虚拟设备音量 * 软件设置的百分比同步到其他设备
            for (device_id, endpoint_volume) in target_volume_controls.iter() {
                let app_volume = shared_volumes
                    .lock()
                    .ok()
                    .and_then(|v| v.get(device_id).copied())
                    .unwrap_or(1.0);
                
                // 目标设备系统音量 = 虚拟设备音量 * 软件音量百分比
                let target_system_volume = system_volume * app_volume;
                let _ = endpoint_volume.SetMasterVolumeLevelScalar(target_system_volume, std::ptr::null());
                // 同步静音状态
                let _ = endpoint_volume.SetMute(is_muted, std::ptr::null());
            }

            let packet_size = capture
                .GetNextPacketSize()
                .map_err(|e| format!("GetNextPacketSize failed: {:?}", e))?;
            if packet_size == 0 {
                continue;
            }

            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            capture
                .GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                .map_err(|e| format!("GetBuffer failed: {:?}", e))?;

            if num_frames == 0 || data_ptr.is_null() {
                let _ = capture.ReleaseBuffer(num_frames);
                continue;
            }

            let data = std::slice::from_raw_parts(
                data_ptr as *const f32,
                (num_frames as usize) * channels as usize,
            );

            // 推入延迟缓冲区（批量推入，减少内存分配）
            for buffer in delay_buffers.values_mut() {
                buffer.push_slice(data);
            }

            // 从延迟缓冲区读取并渲染
            for (device_id, render_client) in render_outputs.iter() {
                if let Some(buffer) = delay_buffers.get_mut(device_id) {
                    if let Ok(out_ptr) = render_client.GetBuffer(num_frames) {
                        if !out_ptr.is_null() {
                            let out_slice = std::slice::from_raw_parts_mut(
                                out_ptr as *mut f32,
                                (num_frames as usize) * channels as usize,
                            );

                            for frame in out_slice.chunks_mut(channels as usize) {
                                let delayed_frame = buffer.pop_or_silent();
                                for (j, sample) in frame.iter_mut().enumerate() {
                                    *sample = delayed_frame.get(j).copied().unwrap_or(0.0);
                                }
                            }
                            let _ = render_client.ReleaseBuffer(num_frames, 0);
                        }
                    }
                }
            }

            let _ = capture.ReleaseBuffer(num_frames);
        }

        // 停止所有渲染客户端
        for client in render_clients.values() {
            let _ = client.Stop();
        }

        Ok(())
    }
}

impl Default for AudioRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioRouter {
    fn drop(&mut self) {
        self.stop();
    }
}

unsafe impl Send for AudioRouter {}
unsafe impl Sync for AudioRouter {}
