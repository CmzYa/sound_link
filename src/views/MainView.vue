<script setup>
import { ref, onMounted, onUnmounted, computed, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-shell";
import { Monitor, Settings, Radio, Route, ExternalLink, RefreshCw } from "lucide-vue-next";
import DeviceBall from "../components/DeviceBall.vue";
import SettingsView from "./SettingsView.vue";

const BALL_SIZE = 44;
const UNSNAP_RADIUS = 85;
const SNAP_RADIUS = 55;
const CONTAINER_SIZE = 280;
const CACHE_EXPIRE_SECONDS = 30;

const allDevices = shallowRef([]);
const activeDeviceId = ref(null);
const showSettings = ref(false);
const advancedMaterial = ref(false);
const isReady = ref(false);
const switchingDeviceId = ref(null);
const appVersion = ref("");
const hasUpdate = ref(false);
const latestVersion = ref("");

// 路由模式相关状态
const isRouterMode = ref(false);
const routerTargetIds = ref([]);
const isRoutingActive = ref(false);
const virtualDeviceStatus = ref({ is_installed: false, device_id: null, device_name: null });
const showInstallDialog = ref(false);
const isRefreshing = ref(false);

// 设备音量和延迟配置
const deviceVolumes = ref({});
const deviceDelays = ref({});

const GITHUB_REPO = "CmzYa/sound_link";

function handleSettingsClose() {
  showSettings.value = false;
}

function handleDeviceSettingsChanged(settings) {
  // 同步设备设置到主界面，确保响应式更新
  deviceVolumes.value = { ...deviceVolumes.value, [settings.deviceId]: settings.volume };
  deviceDelays.value = { ...deviceDelays.value, [settings.deviceId]: settings.delayMs };
}

const devices = computed(() => {
  return allDevices.value.filter(d => 
    !d.name.toLowerCase().includes('cable')
  );
});

const devicePositions = computed(() => {
  const centerX = CONTAINER_SIZE / 2;
  const centerY = CONTAINER_SIZE / 2;
  const total = devices.value.length || 1;
  
  return devices.value.map((device, index) => {
    // 路由模式：检查是否在广播目标列表中
    // 普通模式：检查是否是当前激活设备
    const isSnapped = isRouterMode.value 
      ? routerTargetIds.value.includes(device.id)
      : device.id === activeDeviceId.value;
    
    const baseAngle = (index / total) * 2 * Math.PI;
    const offset = Math.PI / total;
    const angle = baseAngle + offset;
    const radius = isSnapped ? SNAP_RADIUS : UNSNAP_RADIUS;
    
    return {
      x: centerX + Math.cos(angle) * radius - BALL_SIZE / 2,
      y: centerY + Math.sin(angle) * radius - BALL_SIZE / 2
    };
  });
});

function applyData(data) {
  allDevices.value = data.devices;
  advancedMaterial.value = data.config.advanced_material || false;
  
  if (data.default_device_id) {
    activeDeviceId.value = data.default_device_id;
  }
  
  if (data.virtual_device) {
    virtualDeviceStatus.value = data.virtual_device;
  }
}

function isCacheExpired(timestamp) {
  if (!timestamp) return true;
  const now = Math.floor(Date.now() / 1000);
  return (now - timestamp) > CACHE_EXPIRE_SECONDS;
}

async function refreshDevices() {
  try {
    const data = await invoke("get_initial_data");
    applyData(data);
  } catch (e) {
    console.error("Failed to load data:", e);
    allDevices.value = [];
    activeDeviceId.value = null;
  }
}

// 先显示缓存数据，后台异步刷新
async function loadCachedOrRefresh() {
  try {
    const cached = await invoke("get_cached_data");
    if (cached) {
      // 先显示缓存数据，让界面立即响应
      applyData(cached);
      isReady.value = true;
      
      // 如果缓存过期，后台异步刷新
      if (isCacheExpired(cached.timestamp)) {
        refreshDevices();
      }
    } else {
      // 没有缓存，直接刷新
      await refreshDevices();
    }
  } catch (e) {
    console.error("Failed to load cached data:", e);
    await refreshDevices();
  }
}

async function handleDeviceClick(device) {
  if (switchingDeviceId.value) return;
  
  // 路由模式：多选设备加入/移出广播目标
  if (isRouterMode.value) {
    const index = routerTargetIds.value.indexOf(device.id);
    if (index === -1) {
      routerTargetIds.value.push(device.id);
    } else {
      routerTargetIds.value.splice(index, 1);
    }
    
    // 如果正在广播，动态更新广播目标
    if (isRoutingActive.value) {
      try {
        if (routerTargetIds.value.length > 0) {
          const config = {
            devices: routerTargetIds.value.map(id => {
              const d = devices.value.find(dev => dev.id === id);
              return {
                id,
                name: d?.name || "",
                volume: deviceVolumes.value[id] ?? 1.0,
                delay_ms: deviceDelays.value[id] ?? 0,
                enabled: true
              };
            })
          };
          await invoke("update_router_config", { config });
          await invoke("start_routing", { deviceIds: routerTargetIds.value });
        } else {
          await invoke("stop_routing");
          isRoutingActive.value = false;
        }
      } catch (e) {
        console.error("Failed to update routing:", e);
      }
    }
    return;
  }
  
  // 普通模式：单选切换默认设备
  const previousDeviceId = activeDeviceId.value;
  switchingDeviceId.value = device.id;
  
  try {
    activeDeviceId.value = device.id;
    await invoke("set_default_device", { deviceId: device.id });
  } catch (e) {
    console.error("Failed to set device:", e);
    activeDeviceId.value = previousDeviceId;
  } finally {
    switchingDeviceId.value = null;
  }
}

// 切换路由/管理模式
function toggleMode() {
  if (isRouterMode.value) {
    exitRouterMode();
  } else {
    enterRouterMode();
  }
}

// 中心球点击处理
async function handleCenterBallClick() {
  if (switchingDeviceId.value) return;
  
  if (isRouterMode.value) {
    // 路由模式：切换广播开启/关闭
    await toggleRouting();
  }
}

// 进入路由模式
async function enterRouterMode() {
  if (!virtualDeviceStatus.value.is_installed) {
    showInstallDialog.value = true;
    return;
  }
  
  isRouterMode.value = true;
  
  // 并行加载配置和状态
  const [savedConfigResult, statusResult] = await Promise.allSettled([
    invoke("get_saved_router_config"),
    invoke("get_router_status")
  ]);
  
  // 处理保存的设备配置
  if (savedConfigResult.status === 'fulfilled' && savedConfigResult.value?.devices) {
    for (const device of savedConfigResult.value.devices) {
      deviceVolumes.value[device.id] = device.volume;
      deviceDelays.value[device.id] = device.delay_ms;
    }
  } else if (savedConfigResult.status === 'rejected') {
    console.error("Failed to load saved router config:", savedConfigResult.reason);
  }
  
  // 处理路由状态
  if (statusResult.status === 'fulfilled' && statusResult.value?.is_running) {
    isRoutingActive.value = true;
    routerTargetIds.value = statusResult.value.target_devices
      .filter(d => d.enabled && d.id !== virtualDeviceStatus.value.device_id)
      .map(d => d.id);
  } else {
    // 继承当前设备管理模式下选择的设备作为广播目标
    routerTargetIds.value = activeDeviceId.value ? [activeDeviceId.value] : [];
    if (statusResult.status === 'rejected') {
      console.error("Failed to load router status:", statusResult.reason);
    }
  }
}

// 打开 VB-Cable 下载页面
function openDownloadPage() {
  open("https://vb-audio.com/Cable/");
}

// 刷新虚拟设备检测
async function refreshVirtualDevice() {
  isRefreshing.value = true;
  try {
    const status = await invoke("refresh_virtual_device");
    virtualDeviceStatus.value = status;
    if (virtualDeviceStatus.value.is_installed) {
      showInstallDialog.value = false;
    }
  } catch (e) {
    console.error("Failed to refresh virtual device:", e);
  } finally {
    isRefreshing.value = false;
  }
}

// 退出路由模式
async function exitRouterMode() {
  // 如果正在广播，先停止
  if (isRoutingActive.value) {
    try {
      await invoke("stop_routing");
    } catch (e) {
      console.error("Failed to stop routing:", e);
    }
    isRoutingActive.value = false;
  }
  isRouterMode.value = false;
  routerTargetIds.value = [];
}

// 开始广播
async function startRouting() {
  if (routerTargetIds.value.length === 0) return;
  
  const config = {
    devices: routerTargetIds.value.map(id => {
      const device = devices.value.find(d => d.id === id);
      return {
        id,
        name: device?.name || "",
        volume: deviceVolumes.value[id] ?? 1.0,
        delay_ms: deviceDelays.value[id] ?? 0,
        enabled: true
      };
    })
  };
  
  await invoke("update_router_config", { config });
  await invoke("start_routing", { deviceIds: routerTargetIds.value });
  isRoutingActive.value = true;
}

// 切换路由状态（开始/停止广播）
async function toggleRouting() {
  if (switchingDeviceId.value) return;
  switchingDeviceId.value = "routing";
  
  try {
    if (isRoutingActive.value) {
      await invoke("stop_routing");
      isRoutingActive.value = false;
    } else if (routerTargetIds.value.length > 0) {
      await startRouting();
    }
  } catch (e) {
    console.error("Failed to toggle routing:", e);
    alert(e);
  } finally {
    switchingDeviceId.value = null;
  }
}

async function hideWindow() {
  try {
    await invoke("refresh_and_cache");
    await invoke("hide_window");
  } catch (e) {
    console.error("Failed to hide window:", e);
  }
}

function handleAppClick(e) {
  if (!showSettings.value && (e.target.id === "app" || e.target.classList.contains("container"))) {
    hideWindow();
  }
}

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

async function setupThemeListener() {
  let systemAccentColor = null;
  
  try {
    systemAccentColor = await invoke("get_system_accent_color");
  } catch (e) {
    console.error("Failed to get system accent color:", e);
  }
  
  const updateTheme = async () => {
    let systemTheme = null;
    
    try {
      systemTheme = await invoke("get_system_theme");
    } catch (e) {
      console.error("Failed to get system theme:", e);
    }
    
    let isDark;
    
    if (systemTheme !== null) {
      isDark = systemTheme;
    } else {
      isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    
    let themeColor;
    if (systemAccentColor) {
      themeColor = systemAccentColor;
    } else {
      themeColor = isDark ? "#60a5fa" : "#0078d4";
    }
    
    document.documentElement.style.setProperty("--theme-color", themeColor);
    document.documentElement.style.setProperty("--theme-glow", hexToRgba(themeColor, 0.4));
    
    // 路由模式颜色变量
    document.documentElement.style.setProperty("--router-color", "#8b5cf6");
    document.documentElement.style.setProperty("--router-glow", "rgba(139, 92, 246, 0.4)");
    
    // 广播激活颜色变量
    document.documentElement.style.setProperty("--active-color", "#22c55e");
    document.documentElement.style.setProperty("--active-glow", "rgba(34, 197, 94, 0.4)");
    
    if (isDark) {
      document.documentElement.style.setProperty("--glass-bg", "rgba(28, 28, 32, 0.75)");
      document.documentElement.style.setProperty("--glass-border", "rgba(255, 255, 255, 0.08)");
      document.documentElement.style.setProperty("--text-color", "rgba(255, 255, 255, 0.9)");
      document.documentElement.style.setProperty("--text-secondary", "rgba(255, 255, 255, 0.6)");
    } else {
      document.documentElement.style.setProperty("--glass-bg", "rgba(255, 255, 255, 0.75)");
      document.documentElement.style.setProperty("--glass-border", "rgba(0, 0, 0, 0.08)");
      document.documentElement.style.setProperty("--text-color", "rgba(0, 0, 0, 0.9)");
      document.documentElement.style.setProperty("--text-secondary", "rgba(0, 0, 0, 0.6)");
    }
    
    document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
  };
  
  await updateTheme();
  
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", updateTheme);
}

async function loadAppVersion() {
  try {
    appVersion.value = await getVersion();
  } catch (e) {
    console.error("Failed to get version:", e);
  }
}

function compareVersions(current, latest) {
  const currentParts = current.split('.').map(Number);
  const latestParts = latest.split('.').map(Number);
  
  for (let i = 0; i < Math.max(currentParts.length, latestParts.length); i++) {
    const currentPart = currentParts[i] || 0;
    const latestPart = latestParts[i] || 0;
    
    if (latestPart > currentPart) return 1;
    if (latestPart < currentPart) return -1;
  }
  return 0;
}

async function checkForUpdate() {
  try {
    const response = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`);
    if (!response.ok) return;
    
    const release = await response.json();
    const latest = release.tag_name.replace(/^v/, '');
    const currentVersion = appVersion.value || "0.0.0";
    
    if (compareVersions(currentVersion, latest) > 0) {
      hasUpdate.value = true;
      latestVersion.value = latest;
    }
  } catch (e) {
    console.error("Failed to check for updates:", e);
  }
}

let unlisten = null;
let unlistenSettings = null;
let unlistenWindowShown = null;

onMounted(async () => {
  const [, ,] = await Promise.allSettled([
    loadCachedOrRefresh(),
    setupThemeListener(),
    loadAppVersion()
  ]);
  
  isReady.value = true;
  
  checkForUpdate();
  
  // 监听窗口显示事件，异步刷新数据
  unlistenWindowShown = await listen("window-shown", () => {
    loadCachedOrRefresh();
  });
  
  unlisten = await listen("refresh-devices", async () => {
    await refreshDevices();
  });
  
  unlistenSettings = await listen("show-settings", () => {
    showSettings.value = true;
  });
  
  const appWindow = getCurrentWindow();
  appWindow.onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      hideWindow();
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (unlistenSettings) unlistenSettings();
  if (unlistenWindowShown) unlistenWindowShown();
});
</script>

<template>
  <div id="app" :class="{ 'advanced-material': advancedMaterial, 'is-ready': isReady }" @click="handleAppClick">
    <!-- VB-Cable 安装提示弹窗 -->
    <div v-if="showInstallDialog" class="install-dialog-overlay" @click.stop>
      <div class="install-dialog">
        <div class="install-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 2L2 7l10 5 10-5-10-5z"/>
            <path d="M2 17l10 5 10-5"/>
            <path d="M2 12l10 5 10-5"/>
          </svg>
        </div>
        <h3>需要安装虚拟音频设备</h3>
        <p class="install-desc">
          音频路由功能需要 VB-Cable 虚拟音频设备。<br>
          请下载安装后点击刷新检测。
        </p>
        <div class="install-actions">
          <button class="install-btn primary" @click="openDownloadPage">
            <ExternalLink :size="16" />
            打开下载页面
          </button>
          <button class="install-btn" :disabled="isRefreshing" @click="refreshVirtualDevice">
            <RefreshCw :size="16" :class="{ 'spinning': isRefreshing }" />
            {{ isRefreshing ? '检测中...' : '刷新检测' }}
          </button>
        </div>
        <button class="install-close" @click="showInstallDialog = false">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 6L6 18M6 6l12 12"/>
          </svg>
        </button>
      </div>
    </div>
    
    <div v-if="!showSettings" class="top-buttons">
      <button class="mode-btn" :class="{ 'router-mode': isRouterMode }" @click.stop="toggleMode" :title="isRouterMode ? '管理模式' : '路由模式'">
        <Route :size="16" />
      </button>
      <button class="settings-btn" :class="{ 'has-update': hasUpdate }" @click.stop="showSettings = !showSettings">
        <Settings :size="16" />
      </button>
    </div>
    
    <SettingsView 
      v-if="showSettings" 
      :app-version="appVersion"
      :initial-devices="allDevices"
      :initial-advanced-material="advancedMaterial"
      :has-update="hasUpdate"
      :latest-version="latestVersion"
      @close="handleSettingsClose" 
      @config-changed="refreshDevices"
      @device-settings-changed="handleDeviceSettingsChanged"
    />
    
    <template v-else>
      <div class="container">
        <div 
          class="center-ball" 
          :class="{ 
            'advanced-material': advancedMaterial, 
            'router-mode': isRouterMode,
            'routing-active': isRoutingActive,
            'no-vb-cable': !virtualDeviceStatus.is_installed
          }"
          @click="handleCenterBallClick"
        >
          <div class="center-inner">
            <Radio v-if="isRouterMode" :size="26" class="icon" />
            <Monitor v-else :size="26" class="icon" />
          </div>
          <div v-if="isRouterMode" class="router-indicator">
            {{ isRoutingActive ? '停止' : '开始' }}
          </div>
        </div>
        
        <DeviceBall
          v-for="(device, index) in devices"
          :key="device.id"
          :device="device"
          :is-active="isRouterMode ? routerTargetIds.includes(device.id) : device.id === activeDeviceId"
          :is-loading="device.id === switchingDeviceId"
          :position="devicePositions[index]"
          :advanced-material="advancedMaterial"
          :is-router-mode="isRouterMode"
          @click="handleDeviceClick(device)"
        />
        
        <div v-if="devices.length === 0" class="no-device-hint">
          未检测到音频设备
        </div>
        
        <div v-if="isRouterMode" class="router-status">
          <span v-if="isRoutingActive" class="status-active">
            广播中 ({{ routerTargetIds.length }} 设备)
          </span>
          <span v-else-if="routerTargetIds.length > 0" class="status-ready">
            已选择 {{ routerTargetIds.length }} 个设备
          </span>
          <span v-else class="status-hint">
            点击设备选择广播目标
          </span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
#app {
  pointer-events: none;
  opacity: 0.95;
}

#app.is-ready {
  pointer-events: auto;
  opacity: 1;
  transition: opacity 0.15s ease;
}

.container {
  position: relative;
  width: 280px;
  height: 280px;
}

.top-buttons {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 100;
  display: flex;
  gap: 6px;
}

.mode-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--glass-bg);
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  backdrop-filter: blur(10px) saturate(180%);
  -webkit-backdrop-filter: blur(10px) saturate(180%);
  border: 1px solid var(--glass-border);
}

.mode-btn:hover {
  background: color-mix(in srgb, var(--glass-bg) 120%, var(--theme-color));
  color: var(--text-color);
  border-color: var(--theme-color);
}

.mode-btn.router-mode {
  color: #8b5cf6;
  border-color: #8b5cf6;
  background: rgba(139, 92, 246, 0.15);
}

.settings-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--glass-bg);
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  backdrop-filter: blur(10px) saturate(180%);
  -webkit-backdrop-filter: blur(10px) saturate(180%);
  border: 1px solid var(--glass-border);
}

.settings-btn:hover {
  background: color-mix(in srgb, var(--glass-bg) 120%, var(--theme-color));
  color: var(--text-color);
  border-color: var(--theme-color);
}

.settings-btn.has-update {
  color: #22c55e;
  border-color: #22c55e;
  background: rgba(34, 197, 94, 0.15);
}

.center-ball {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 60px;
  height: 60px;
  z-index: 10;
  will-change: transform;
  transform: translate3d(-50%, -50%, 0);
  backface-visibility: hidden;
  cursor: pointer;
}

.center-inner {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: linear-gradient(145deg, 
    var(--theme-color), 
    color-mix(in srgb, var(--theme-color) 55%, black)
  );
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 
    0 4px 20px rgba(0, 0, 0, 0.4),
    0 0 25px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  will-change: auto;
  transform: translate3d(0, 0, 0);
}

/* 深色模式 - 高级材质中心球 */
.center-ball.advanced-material {
  position: relative;
}

.center-ball.advanced-material::before {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 100%;
  height: 100%;
  background: var(--theme-glow);
  border-radius: 50%;
  transform: translate(-50%, -50%) scale(2);
  opacity: 0.3;
  z-index: -1;
  filter: blur(10px);
}

/* 深色模式 - 高级材质中心球 */
.center-ball.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--theme-color) 65%, white),
    color-mix(in srgb, var(--theme-color) 40%, rgba(255, 255, 255, 0.2))
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.3) !important;
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 25px var(--theme-glow) !important;
}

.center-ball .icon {
  color: white;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.4));
}

.no-device-hint {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  color: var(--text-secondary);
  font-size: 11px;
  text-align: center;
  margin-top: 70px;
  line-height: 1.6;
}

/* ========== 浅色模式 ========== */

/* 浅色模式 - 中心球 */
[data-theme="light"] .center-ball .center-inner {
  background: linear-gradient(145deg, 
    var(--theme-color), 
    color-mix(in srgb, var(--theme-color) 75%, white)
  );
  box-shadow: 
    0 4px 20px rgba(0, 0, 0, 0.12),
    0 0 25px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.35);
}

/* 浅色模式 - 高级材质中心球 */
[data-theme="light"] .center-ball.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--theme-color) 85%, white),
    color-mix(in srgb, var(--theme-color) 65%, white)
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.6) !important;
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 25px var(--theme-glow) !important;
}

[data-theme="light"] .center-ball.advanced-material::before,
[data-theme="light"] .center-ball.advanced-material::after {
  filter: blur(15px);
  opacity: 0.7;
}

/* 路由模式样式 */
.center-ball.router-mode .center-inner {
  background: linear-gradient(145deg,
    var(--router-color),
    color-mix(in srgb, var(--router-color) 65%, black)
  ) !important;
}

/* 路由模式 - 高级材质 */
.center-ball.router-mode.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--router-color) 65%, white),
    color-mix(in srgb, var(--router-color) 40%, rgba(255, 255, 255, 0.2))
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.3) !important;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 25px var(--router-glow) !important;
}

.center-ball.routing-active .center-inner {
  background: linear-gradient(145deg,
    var(--active-color),
    color-mix(in srgb, var(--active-color) 65%, black)
  ) !important;
  animation: pulse 1.5s ease-in-out infinite;
}

/* 广播激活 - 高级材质 */
.center-ball.routing-active.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--active-color) 65%, white),
    color-mix(in srgb, var(--active-color) 40%, rgba(255, 255, 255, 0.2))
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.3) !important;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 25px var(--active-glow) !important;
}

.center-ball.no-vb-cable .center-inner {
  background: linear-gradient(145deg, 
    #f59e0b, 
    color-mix(in srgb, #f59e0b 65%, black)
  );
}

@keyframes pulse {
  0%, 100% { box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4), 0 0 25px var(--active-glow); }
  50% { box-shadow: 0 4px 30px rgba(34, 197, 94, 0.6), 0 0 40px var(--active-glow); }
}

.router-indicator {
  position: absolute;
  bottom: -24px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 10px;
  color: var(--text-secondary);
  white-space: nowrap;
  pointer-events: none;
}

.router-status {
  position: absolute;
  bottom: -45px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  white-space: nowrap;
  pointer-events: none;
}

.router-status .status-active {
  color: #22c55e;
}

.router-status .status-ready {
  color: #8b5cf6;
}

.router-status .status-hint {
  color: var(--text-tertiary);
}

/* 浅色模式 - 路由模式 */
[data-theme="light"] .center-ball.router-mode .center-inner {
  background: linear-gradient(145deg,
    var(--router-color),
    color-mix(in srgb, var(--router-color) 75%, white)
  ) !important;
}

/* 浅色模式 - 路由模式高级材质 */
[data-theme="light"] .center-ball.router-mode.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--router-color) 85%, white),
    color-mix(in srgb, var(--router-color) 65%, white)
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.6) !important;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 25px var(--router-glow) !important;
}

[data-theme="light"] .center-ball.routing-active .center-inner {
  background: linear-gradient(145deg,
    var(--active-color),
    color-mix(in srgb, var(--active-color) 75%, white)
  ) !important;
}

/* 浅色模式 - 广播激活高级材质 */
[data-theme="light"] .center-ball.routing-active.advanced-material .center-inner {
  background: linear-gradient(145deg,
    color-mix(in srgb, var(--active-color) 85%, white),
    color-mix(in srgb, var(--active-color) 65%, white)
  ) !important;
  border: 1px solid rgba(255, 255, 255, 0.6) !important;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 25px var(--active-glow) !important;
}

/* 安装提示弹窗 */
.install-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.install-dialog {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 16px;
  padding: 24px;
  max-width: 320px;
  text-align: center;
  position: relative;
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.install-icon {
  width: 72px;
  height: 72px;
  margin: 0 auto 16px;
  border-radius: 50%;
  background: linear-gradient(145deg, #f59e0b, #d97706);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}

.install-dialog h3 {
  margin: 0 0 12px;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-color);
}

.install-desc {
  margin: 0 0 20px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.install-actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.install-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 16px;
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  background: var(--glass-bg);
  color: var(--text-color);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.install-btn:hover:not(:disabled) {
  border-color: var(--theme-color);
  background: color-mix(in srgb, var(--glass-bg) 120%, var(--theme-color));
}

.install-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.install-btn.primary {
  background: linear-gradient(145deg, #f59e0b, #d97706);
  border-color: transparent;
  color: white;
}

.install-btn.primary:hover {
  background: linear-gradient(145deg, #fbbf24, #f59e0b);
}

.install-close {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.install-close:hover {
  background: var(--glass-border);
  color: var(--text-color);
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
