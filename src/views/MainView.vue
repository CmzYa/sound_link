<script setup>
import { ref, onMounted, onUnmounted, computed, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { Monitor, Settings } from "lucide-vue-next";
import DeviceBall from "../components/DeviceBall.vue";
import SettingsView from "./SettingsView.vue";

const BALL_SIZE = 44;
const UNSNAP_RADIUS = 85;
const SNAP_RADIUS = 55;
const CONTAINER_SIZE = 280;
const CACHE_EXPIRE_SECONDS = 30;

const allDevices = shallowRef([]);
const activeDeviceId = ref(null);
const configDefaultDeviceId = ref(null);
const showSettings = ref(false);
const advancedMaterial = ref(false);
const isReady = ref(false);
const switchingDeviceId = ref(null);
const appVersion = ref("");
const hasUpdate = ref(false);
const latestVersion = ref("");

const GITHUB_REPO = "CmzYa/sound_link";

function handleSettingsClose() {
  showSettings.value = false;
}

const devices = computed(() => {
  return allDevices.value.filter(d => d.id !== configDefaultDeviceId.value);
});

const devicePositions = computed(() => {
  const centerX = CONTAINER_SIZE / 2;
  const centerY = CONTAINER_SIZE / 2;
  const total = devices.value.length || 1;
  
  return devices.value.map((device, index) => {
    const isActive = device.id === activeDeviceId.value;
    const baseAngle = (index / total) * 2 * Math.PI;
    const offset = Math.PI / total;
    const angle = baseAngle + offset;
    const radius = isActive ? SNAP_RADIUS : UNSNAP_RADIUS;
    
    return {
      x: centerX + Math.cos(angle) * radius - BALL_SIZE / 2,
      y: centerY + Math.sin(angle) * radius - BALL_SIZE / 2
    };
  });
});

function applyData(data) {
  allDevices.value = data.devices;
  activeDeviceId.value = data.default_device_id;
  configDefaultDeviceId.value = data.config.default_device_id;
  advancedMaterial.value = data.config.advanced_material || false;
  
  if (activeDeviceId.value === configDefaultDeviceId.value) {
    activeDeviceId.value = null;
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

async function loadCachedOrRefresh() {
  try {
    const cached = await invoke("get_cached_data");
    if (cached && !isCacheExpired(cached.timestamp)) {
      applyData(cached);
    } else {
      await refreshDevices();
    }
  } catch (e) {
    console.error("Failed to load cached data:", e);
    await refreshDevices();
  }
}

async function handleDeviceClick(device) {
  if (switchingDeviceId.value) return;
  
  const previousDeviceId = activeDeviceId.value;
  switchingDeviceId.value = device.id;
  
  try {
    if (device.id === activeDeviceId.value) {
      // 未设置默认设备时，禁止取消选择
      if (!configDefaultDeviceId.value) {
        return;
      }
      activeDeviceId.value = null;
      await invoke("set_default_device", { deviceId: configDefaultDeviceId.value });
    } else {
      activeDeviceId.value = device.id;
      await invoke("set_default_device", { deviceId: device.id });
    }
  } catch (e) {
    console.error("Failed to set device:", e);
    activeDeviceId.value = previousDeviceId;
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

onMounted(async () => {
  const [, ,] = await Promise.allSettled([
    loadCachedOrRefresh(),
    setupThemeListener(),
    loadAppVersion()
  ]);
  
  isReady.value = true;
  
  checkForUpdate();
  
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
});
</script>

<template>
  <div id="app" :class="{ 'advanced-material': advancedMaterial, 'is-ready': isReady }" @click="handleAppClick">
    <button v-if="!showSettings" class="settings-btn" :class="{ 'has-update': hasUpdate }" @click.stop="showSettings = !showSettings">
      <Settings :size="16" />
    </button>
    
    <SettingsView 
      v-if="showSettings" 
      :app-version="appVersion"
      :initial-devices="allDevices"
      :initial-default-device-id="configDefaultDeviceId"
      :initial-advanced-material="advancedMaterial"
      :has-update="hasUpdate"
      :latest-version="latestVersion"
      @close="handleSettingsClose" 
      @config-changed="refreshDevices" 
    />
    
    <template v-else>
      <div class="container">
        <div class="center-ball" :class="{ 'advanced-material': advancedMaterial }">
          <div class="center-inner">
            <Monitor :size="26" class="icon" />
          </div>
        </div>
        
        <DeviceBall
          v-for="(device, index) in devices"
          :key="device.id"
          :device="device"
          :is-active="device.id === activeDeviceId"
          :is-loading="device.id === switchingDeviceId"
          :position="devicePositions[index]"
          :advanced-material="advancedMaterial"
          @click="handleDeviceClick(device)"
        />
        
        <div v-if="devices.length === 0" class="no-device-hint">
          未检测到音频设备
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

.settings-btn {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 100;
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
  pointer-events: none;
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

.center-ball.advanced-material .center-inner {
  background: linear-gradient(145deg, 
    color-mix(in srgb, var(--theme-color) 65%, white), 
    color-mix(in srgb, var(--theme-color) 40%, rgba(255, 255, 255, 0.2))
  );
  border: 1px solid rgba(255, 255, 255, 0.25);
  box-shadow: 
    0 8px 35px rgba(0, 0, 0, 0.3),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 30px var(--theme-glow);
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
  );
  border: 1px solid rgba(255, 255, 255, 0.6);
  box-shadow: 
    0 8px 35px rgba(0, 0, 0, 0.1),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 30px var(--theme-glow);
}

[data-theme="light"] .center-ball.advanced-material::before,
[data-theme="light"] .center-ball.advanced-material::after {
  filter: blur(15px);
  opacity: 0.7;
}
</style>
