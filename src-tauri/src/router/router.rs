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
use super::escape_powershell_string;
use super::volume_sync::VolumeSync;
use crate::router::{RouterConfig, RouterStatus, ValidationResult, VirtualDeviceStatus};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 音频路由器 - 将虚拟设备音频分发到多个目标设备
pub struct AudioRouter {
    config: RouterConfig,                           // 路由配置
    source_device_id: Option<String>,               // 源设备ID (VB-Cable)
    source_device_name: Option<String>,             // 源设备名称
    running: Arc<AtomicBool>,                       // 运行标志
    thread_handle: Option<JoinHandle<()>>,          // 线程句柄
    delay_buffers: HashMap<String, DelayBuffer>,    // 延迟缓冲区
    sample_rate: u32,                               // 采样率
    channels: u32,                                  // 通道数
    vb_cable_id: Option<String>,                    // VB-Cable ID
    original_default_device_id: Option<String>,     // 原默认设备ID
    shared_volumes: Arc<Mutex<HashMap<String, f32>>>,   // 共享音量设置
    shared_delays: Arc<Mutex<HashMap<String, u32>>>,    // 共享延迟设置
    last_error: Arc<Mutex<Option<String>>>,         // 最近一次路由错误
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
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    /// 查找 VB-Cable 虚拟设备
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
            let device_id = stdout.trim().to_string();
            if !device_id.is_empty() && !device_id.starts_with("Active code page:") {
                return Some(device_id);
            }
        }
        None
    }

    pub fn get_virtual_device_status(&self) -> VirtualDeviceStatus {
        VirtualDeviceStatus {
            is_installed: self.vb_cable_id.is_some(),
            device_id: self.vb_cable_id.clone(),
            device_name: self.vb_cable_id.as_ref().and_then(|id| Self::get_device_name_by_id(id)),
        }
    }

    pub fn refresh_virtual_device(&mut self) -> VirtualDeviceStatus {
        self.vb_cable_id = Self::find_vb_cable_device();
        self.get_virtual_device_status()
    }

    fn get_device_name_by_id(device_id: &str) -> Option<String> {
        let safe_id = escape_powershell_string(device_id);

        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-Command",
            &format!("chcp 65001 > $null; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; (Get-AudioDevice -Id '{}').Name", safe_id)
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

            let device_id = device.GetId()
                .map_err(|e| format!("Failed to get device id: {:?}", e))?;

            Ok(device_id.to_string()
                .map_err(|e| format!("Failed to convert id: {:?}", e))?)
        }
    }

    pub fn validate_targets(&self, device_ids: &[String]) -> ValidationResult {
        let default_device_id = self.get_default_output_device_id().ok();

        let conflicts: Vec<String> = device_ids
            .iter()
            .filter(|id| Some(id.as_str()) == default_device_id.as_deref())
            .cloned()
            .collect();

        ValidationResult {
            has_conflicts: !conflicts.is_empty(),
            conflict_devices: conflicts,
            warning: if default_device_id.is_some() {
                "源设备将被自动排除以避免回音".to_string()
            } else {
                String::new()
            },
        }
    }

    /// 启动路由
    pub fn start(&mut self, target_device_ids: Vec<String>) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            self.stop();
            thread::sleep(Duration::from_millis(50));
        }

        let vb_cable_id = self.vb_cable_id.clone()
            .ok_or("未检测到 VB-Cable 虚拟设备，请先安装 VB-Cable")?;

        // 保存并切换默认设备
        let current_default = self.get_default_output_device_id()?;
        self.original_default_device_id = Some(current_default.clone());

        if current_default != vb_cable_id {
            self.set_default_device(&vb_cable_id)?;
        }

        self.source_device_id = Some(vb_cable_id.clone());
        self.source_device_name = Self::get_device_name_by_id(&vb_cable_id);

        // 过滤有效目标设备
        let valid_target_ids: Vec<String> = target_device_ids
            .into_iter()
            .filter(|id| id != &vb_cable_id)
            .collect();

        if valid_target_ids.is_empty() {
            if let Some(original) = &self.original_default_device_id {
                let _ = self.set_default_device(original);
            }
            return Err("没有有效的目标设备".to_string());
        }

        // 初始化延迟缓冲区
        self.delay_buffers.clear();
        for device in &self.config.devices {
            if valid_target_ids.contains(&device.id) && device.enabled {
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
                if valid_target_ids.contains(&device.id) && device.enabled {
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
        let last_error = self.last_error.clone();

        let handle = thread::spawn(move || unsafe {
            if let Err(e) = Self::router_loop(vb_cable_id, valid_target_ids, config, running, shared_volumes, shared_delays) {
                eprintln!("Router loop error: {}", e);
                if let Ok(mut err) = last_error.lock() {
                    *err = Some(e);
                }
            }
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    fn set_default_device(&self, device_id: &str) -> Result<(), String> {
        let safe_id = escape_powershell_string(device_id);

        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command",
            &format!("Set-AudioDevice -Id '{}' -Default", safe_id),
        ]);

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd.output().map_err(|e| format!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to set default device: {}", stderr));
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.delay_buffers.clear();

        if let Some(original_id) = &self.original_default_device_id {
            let _ = self.set_default_device(original_id);
        }
        self.original_default_device_id = None;
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取最近一次路由错误（消费后清除）
    pub fn take_last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|mut e| e.take())
    }

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

    pub fn set_device_volume(&mut self, device_id: &str, volume: f32) {
        if let Some(device) = self.config.devices.iter_mut().find(|d| d.id == device_id) {
            device.volume = volume;
        }
        if let Ok(mut volumes) = self.shared_volumes.lock() {
            volumes.insert(device_id.to_string(), volume);
        }
    }

    pub fn set_device_delay(&mut self, device_id: &str, delay_ms: u32) {
        if let Some(buffer) = self.delay_buffers.get_mut(device_id) {
            buffer.set_delay(delay_ms, self.sample_rate);
        }
        if let Some(device) = self.config.devices.iter_mut().find(|d| d.id == device_id) {
            device.delay_ms = delay_ms;
        }
        if let Ok(mut delays) = self.shared_delays.lock() {
            delays.insert(device_id.to_string(), delay_ms);
        }
    }

    pub fn update_config(&mut self, config: RouterConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &RouterConfig {
        &self.config
    }

    /// 路由主循环：捕获虚拟设备音频并分发到目标设备
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

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create enumerator: {:?}", e))?;

        // 初始化捕获设备
        let source_id = windows::core::HSTRING::from(source_device_id.as_str());
        let capture_device = enumerator.GetDevice(&source_id)
            .map_err(|e| format!("Failed to get VB-Cable device: {:?}", e))?;

        let capture_client: IAudioClient = capture_device.Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to activate capture client: {:?}", e))?;

        let format_ptr = capture_client.GetMixFormat()
            .map_err(|e| format!("Failed to get format: {:?}", e))?;

        let wave_format = &*format_ptr;
        let sample_rate = wave_format.nSamplesPerSec;
        let channels = wave_format.nChannels as u32;

        // Loopback 模式捕获
        capture_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            10_000_000, 0, format_ptr, None,
        ).map_err(|e| format!("Failed to initialize capture: {:?}", e))?;

        let capture: IAudioCaptureClient = capture_client.GetService()
            .map_err(|e| format!("Failed to get capture client: {:?}", e))?;
        capture_client.Start()
            .map_err(|e| format!("Failed to start capture: {:?}", e))?;
        defer! { let _ = capture_client.Stop(); }

        // 获取源设备音量控制接口
        let source_volume_control: IAudioEndpointVolume = capture_device.Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to get source volume interface: {:?}", e))?;

        // 初始化渲染客户端
        let mut render_clients: HashMap<String, IAudioClient> = HashMap::new();
        let mut render_outputs: HashMap<String, IAudioRenderClient> = HashMap::new();
        let mut delay_buffers: HashMap<String, DelayBuffer> = HashMap::new();
        let mut target_channels_map: HashMap<String, usize> = HashMap::new();  // 目标设备通道数
        let mut volume_sync = VolumeSync::new(shared_volumes.clone());

        println!("[Router] ┌─────────────────────────────────────────┐");
        println!("[Router] │          开始初始化渲染设备             │");
        println!("[Router] └─────────────────────────────────────────┘");
        let mut device_index = 0u32;

        for device_config in &config.devices {
            if !target_ids.contains(&device_config.id) || !device_config.enabled {
                continue;
            }
            
            device_index += 1;
            let target_id = windows::core::HSTRING::from(device_config.id.as_str());
            
            if let Ok(target_device) = enumerator.GetDevice(&target_id) {
                println!("[Router]");
                println!("[Router] ▸ #{} {}", device_index, device_config.name);
                
                // 音量控制接口
                if let Ok(target_volume_control) = target_device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    volume_sync.add_target(device_config.id.clone(), target_volume_control);
                }

                // 渲染客户端 - 使用目标设备自己的格式
                match target_device.Activate::<IAudioClient>(CLSCTX_ALL, None) {
                    Ok(audio_client) => {
                        // 获取目标设备的混合格式
                        match audio_client.GetMixFormat() {
                            Ok(target_format) => {
                                let target_wave = &*target_format;
                                let target_sample_rate = target_wave.nSamplesPerSec;
                                let target_channels = target_wave.nChannels;
                                let target_bits = target_wave.wBitsPerSample;
                                println!("[Router]   格式: {}Hz / {}ch / {}bit", 
                                    target_sample_rate, target_channels, target_bits);
                                
                                // 尝试初始化：先尝试带自动转换，失败则尝试不带
                                let init_result = audio_client.Initialize(
                                    AUDCLNT_SHAREMODE_SHARED,
                                    AUDCLNT_STREAMFLAGS_NOPERSIST | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                                    10_000_000, 0, target_format, None,
                                ).or_else(|_| {
                                    // 备用方案：不使用自动转换标志
                                    audio_client.Initialize(
                                        AUDCLNT_SHAREMODE_SHARED,
                                        AUDCLNT_STREAMFLAGS_NOPERSIST,
                                        10_000_000, 0, target_format, None,
                                    )
                                }).or_else(|_| {
                                    // 备用方案2：使用更小的缓冲区
                                    audio_client.Initialize(
                                        AUDCLNT_SHAREMODE_SHARED,
                                        AUDCLNT_STREAMFLAGS_NOPERSIST,
                                        5_000_000, 0, target_format, None,
                                    )
                                });
                                
                                match init_result {
                                    Ok(_) => {
                                        let target_channels = target_wave.nChannels as usize;
                                        
                                        match audio_client.GetService::<IAudioRenderClient>() {
                                            Ok(render_client) => {
                                                let _ = audio_client.Start();
                                                render_clients.insert(device_config.id.clone(), audio_client);
                                                render_outputs.insert(device_config.id.clone(), render_client);
                                                delay_buffers.insert(
                                                    device_config.id.clone(),
                                                    DelayBuffer::new(device_config.delay_ms, sample_rate, target_channels),
                                                );
                                                target_channels_map.insert(device_config.id.clone(), target_channels);
                                                println!("[Router]   状态: ✓ 初始化成功");
                                            }
                                            Err(e) => println!("[Router]   状态: ✗ 渲染客户端获取失败 {:?}", e),
                                        }
                                    }
                                    Err(e) => println!("[Router]   状态: ✗ 音频客户端初始化失败 {:?}", e),
                                }
                            }
                            Err(e) => println!("[Router]   状态: ✗ 混合格式获取失败 {:?}", e),
                        }
                    }
                    Err(e) => println!("[Router]   状态: ✗ 音频客户端激活失败 {:?}", e),
                }
                
                // 蓝牙设备初始化间隔，避免资源竞争
                thread::sleep(Duration::from_millis(100));
            } else {
                println!("[Router]   状态: ✗ 设备获取失败");
            }
        }
        
        println!("[Router]");
        println!("[Router] ┌─────────────────────────────────────────┐");
        println!("[Router] │  初始化完成: 成功 {} 个 / 总计 {} 个      │", render_clients.len(), device_index);
        println!("[Router] └─────────────────────────────────────────┘");

        // 主循环
        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));

            // 更新延迟设置
            if let Ok(delays) = shared_delays.lock() {
                for (device_id, &delay_ms) in delays.iter() {
                    if let Some(buffer) = delay_buffers.get_mut(device_id) {
                        buffer.set_delay(delay_ms, sample_rate);
                    }
                }
            }

            // 双向音量同步
            volume_sync.sync(&source_volume_control);

            // 获取音频数据
            let packet_size = capture.GetNextPacketSize()
                .map_err(|e| format!("GetNextPacketSize failed: {:?}", e))?;
            if packet_size == 0 {
                continue;
            }

            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            capture.GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
                .map_err(|e| format!("GetBuffer failed: {:?}", e))?;

            if num_frames == 0 || data_ptr.is_null() {
                let _ = capture.ReleaseBuffer(num_frames);
                continue;
            }

            let audio_samples = std::slice::from_raw_parts(
                data_ptr as *const f32,
                (num_frames as usize) * channels as usize,
            );

            // 推入延迟缓冲区
            for buffer in delay_buffers.values_mut() {
                buffer.push_slice(audio_samples);
            }

            // 渲染到目标设备
            for (device_id, render_client) in render_outputs.iter() {
                if let Some(buffer) = delay_buffers.get_mut(device_id) {
                    if let Some(&target_ch) = target_channels_map.get(device_id) {
                        if let Ok(out_ptr) = render_client.GetBuffer(num_frames) {
                            if !out_ptr.is_null() {
                                let out_samples = std::slice::from_raw_parts_mut(
                                    out_ptr as *mut f32,
                                    (num_frames as usize) * target_ch,
                                );

                                for frame in out_samples.chunks_mut(target_ch) {
                                    let delayed_frame = buffer.pop_or_silent();
                                    // 通道映射：将源通道映射到目标通道
                                    for (ch, sample) in frame.iter_mut().enumerate() {
                                        *sample = delayed_frame.get(ch % channels as usize).copied().unwrap_or(0.0);
                                    }
                                }
                                let _ = render_client.ReleaseBuffer(num_frames, 0);
                            }
                        }
                    }
                }
            }

            let _ = capture.ReleaseBuffer(num_frames);
        }

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

// Safety: AudioRouter 的所有字段在跨线程访问时均通过 Mutex 保护。
// running 使用 AtomicBool，是线程安全的。
// shared_volumes 和 shared_delays 使用 Arc<Mutex<...>>，是线程安全的。
// thread_handle 仅在持有 Mutex 时访问。
// AudioRouter 本身不直接被多线程共享，而是通过 AppState 中的 Mutex 间接访问。
unsafe impl Send for AudioRouter {}
unsafe impl Sync for AudioRouter {}
