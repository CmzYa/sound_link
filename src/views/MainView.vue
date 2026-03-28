<script setup>
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Monitor, Settings } from "lucide-vue-next";
import DeviceBall from "../components/DeviceBall.vue";
import SettingsView from "./SettingsView.vue";

const BALL_SIZE = 44;
const UNSNAP_RADIUS = 75;
const SNAP_RADIUS = 53;
const CONTAINER_SIZE = 280;

const allDevices = ref([]);
const activeDeviceId = ref(null);
const configDefaultDeviceId = ref(null);
const showSettings = ref(false);
const advancedMaterial = ref(false);

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

async function refreshDevices() {
  try {
    const data = await invoke("get_initial_data");
    allDevices.value = data.devices;
    activeDeviceId.value = data.default_device_id;
    configDefaultDeviceId.value = data.config.default_device_id;
    advancedMaterial.value = data.config.advanced_material || false;
    
    if (activeDeviceId.value === configDefaultDeviceId.value) {
      activeDeviceId.value = null;
    }
  } catch (e) {
    console.error("Failed to load data:", e);
    allDevices.value = [];
    activeDeviceId.value = null;
  }
}

async function handleDeviceClick(device) {
  const previousDeviceId = activeDeviceId.value;
  
  if (device.id === activeDeviceId.value) {
    activeDeviceId.value = null;
    if (configDefaultDeviceId.value) {
      invoke("set_default_device", { deviceId: configDefaultDeviceId.value }).catch(e => {
        console.error("Failed to set default device:", e);
        activeDeviceId.value = previousDeviceId;
      });
    }
  } else {
    activeDeviceId.value = device.id;
    invoke("set_default_device", { deviceId: device.id }).catch(e => {
      console.error("Failed to set device:", e);
      activeDeviceId.value = previousDeviceId;
    });
  }
}

async function hideWindow() {
  try {
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

let unlisten = null;
let unlistenSettings = null;

onMounted(async () => {
  await refreshDevices();
  await setupThemeListener();
  
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
  <div id="app" :class="{ 'advanced-material': advancedMaterial }" @click="handleAppClick">
    <button v-if="!showSettings" class="settings-btn" @click.stop="showSettings = !showSettings">
      <Settings :size="16" />
    </button>
    
    <SettingsView 
      v-if="showSettings" 
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
          :position="devicePositions[index]"
          :advanced-material="advancedMaterial"
          @click="handleDeviceClick(device)"
        />
        
        <div v-if="devices.length === 0" class="no-device-hint">
          未检测到音频设备<br>请检查 AudioDeviceCmdlets 模块
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
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

.center-ball {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 60px;
  height: 60px;
  z-index: 10;
  will-change: transform;
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
  animation: center-glow 3s ease-in-out infinite;
  will-change: box-shadow;
  transform: translateZ(0);
}

/* 深色模式 - 高级材质中心球 */
.center-ball.advanced-material {
  position: relative;
}

.center-ball.advanced-material::before,
.center-ball.advanced-material::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 100%;
  height: 100%;
  background: var(--theme-glow);
  border-radius: 50%;
  transform: translate(-50%, -50%) scale(0);
  opacity: 1;
  z-index: -1;
  filter: blur(10px);
}

.center-ball.advanced-material::before {
  animation: light-wave 3s ease-out infinite;
}

.center-ball.advanced-material::after {
  animation: light-wave 3s ease-out 1s infinite;
  background: color-mix(in srgb, var(--theme-color) 40%, transparent);
}

.center-ball.advanced-material .center-inner {
  background: linear-gradient(145deg, 
    color-mix(in srgb, var(--theme-color) 65%, white), 
    color-mix(in srgb, var(--theme-color) 40%, rgba(255, 255, 255, 0.2))
  );
  border: 1px solid rgba(255, 255, 255, 0.25);
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
  box-shadow: 
    0 8px 35px rgba(0, 0, 0, 0.3),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 30px var(--theme-glow);
  animation: center-glow-advanced 3s ease-in-out infinite;
}

.center-ball .icon {
  color: white;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.4));
}

@keyframes center-glow {
  0%, 100% {
    box-shadow: 
      0 4px 20px rgba(0, 0, 0, 0.4),
      0 0 25px var(--theme-glow),
      inset 0 1px 0 rgba(255, 255, 255, 0.15);
  }
  50% {
    box-shadow: 
      0 6px 28px rgba(0, 0, 0, 0.5),
      0 0 40px var(--theme-glow),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }
}

@keyframes center-glow-advanced {
  0%, 100% {
    box-shadow: 
      0 8px 35px rgba(0, 0, 0, 0.3),
      inset 0 2px 0 rgba(255, 255, 255, 0.35),
      0 0 30px var(--theme-glow);
  }
  50% {
    box-shadow: 
      0 10px 45px rgba(0, 0, 0, 0.4),
      inset 0 2px 0 rgba(255, 255, 255, 0.4),
      0 0 50px var(--theme-glow);
  }
}

@keyframes light-wave {
  0% {
    transform: translate(-50%, -50%) scale(0);
    opacity: 0.9;
  }
  100% {
    transform: translate(-50%, -50%) scale(3.5);
    opacity: 0;
  }
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
