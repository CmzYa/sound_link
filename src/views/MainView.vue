<script setup>
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Monitor, Settings } from "lucide-vue-next";
import DeviceBall from "../components/DeviceBall.vue";
import SettingsView from "./SettingsView.vue";

const BALL_SIZE = 44;
const CENTER_SIZE = 60;
const UNSNAP_RADIUS = 75;
const SNAP_RADIUS = 53;

const allDevices = ref([]);
const activeDeviceId = ref(null);
const containerRef = ref(null);
const configDefaultDeviceId = ref(null);
const showSettings = ref(false);
const advancedMaterial = ref(false);

function handleSettingsClose() {
  showSettings.value = false;
}

const devices = computed(() => {
  return allDevices.value.filter(d => d.id !== configDefaultDeviceId.value);
});

async function refreshDevices() {
  try {
    allDevices.value = await invoke("get_audio_devices");
  } catch (e) {
    console.error("Failed to load devices:", e);
    allDevices.value = [];
  }
  
  try {
    activeDeviceId.value = await invoke("get_default_device");
  } catch (e) {
    console.error("Failed to load active device:", e);
    activeDeviceId.value = null;
  }
  
  try {
    const config = await invoke("get_config");
    configDefaultDeviceId.value = config.default_device_id;
    advancedMaterial.value = config.advanced_material || false;
  } catch (e) {
    console.error("Failed to load config:", e);
  }
  
  if (activeDeviceId.value === configDefaultDeviceId.value) {
    activeDeviceId.value = null;
  }
}

function getDevicePosition(device, index) {
  const container = containerRef.value;
  if (!container) return { x: 0, y: 0 };
  
  const rect = container.getBoundingClientRect();
  const centerX = rect.width / 2;
  const centerY = rect.height / 2;
  const isActive = device.id === activeDeviceId.value;
  const total = devices.value.length || 1;
  
  const baseAngle = (index / total) * 2 * Math.PI;
  const offset = Math.PI / total;
  const angle = baseAngle + offset;
  
  const radius = isActive ? SNAP_RADIUS : UNSNAP_RADIUS;
  
  return {
    x: centerX + Math.cos(angle) * radius - BALL_SIZE / 2,
    y: centerY + Math.sin(angle) * radius - BALL_SIZE / 2
  };
}

async function handleDeviceClick(device) {
  if (device.id === activeDeviceId.value) {
    activeDeviceId.value = null;
    if (configDefaultDeviceId.value) {
      try {
        await invoke("set_default_device", { deviceId: configDefaultDeviceId.value });
      } catch (e) {
        console.error("Failed to set default device:", e);
      }
    }
  } else {
    activeDeviceId.value = device.id;
    try {
      await invoke("set_default_device", { deviceId: device.id });
    } catch (e) {
      console.error("Failed to set device:", e);
    }
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
  
  const updateTheme = () => {
    const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    
    let themeColor;
    if (systemAccentColor) {
      themeColor = systemAccentColor;
    } else {
      themeColor = isDark ? "#60a5fa" : "#0078d4";
    }
    
    document.documentElement.style.setProperty("--theme-color", themeColor);
    document.documentElement.style.setProperty("--theme-glow", hexToRgba(themeColor, 0.4));
  };
  
  updateTheme();
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
      <div class="container" ref="containerRef">
        <div class="center-ball" :class="{ 'advanced-material': advancedMaterial }">
          <div class="center-inner">
            <Monitor :size="26" class="icon" />
          </div>
        </div>
        
        <DeviceBall
          v-for="(device, index) in devices"
          :key="device.id"
          :device="device"
          :index="index"
          :is-active="device.id === activeDeviceId"
          :position="getDevicePosition(device, index)"
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
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.6);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  backdrop-filter: blur(10px) saturate(180%);
  -webkit-backdrop-filter: blur(10px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.settings-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  color: rgba(255, 255, 255, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
}

.center-ball {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 60px;
  height: 60px;
  z-index: 10;
}

.center-inner {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--theme-color), color-mix(in srgb, var(--theme-color) 50%, black));
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 
    0 4px 20px rgba(0, 0, 0, 0.3),
    0 0 30px var(--theme-glow);
  animation: center-glow 3s ease-in-out infinite;
}

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
  filter: blur(8px);
}

.center-ball.advanced-material::before {
  animation: light-wave 3s ease-out infinite;
}

.center-ball.advanced-material::after {
  animation: light-wave 3s ease-out 1s infinite;
  background: color-mix(in srgb, var(--theme-color) 30%, transparent);
}

.center-ball.advanced-material .center-inner {
  background: linear-gradient(135deg, 
    color-mix(in srgb, var(--theme-color) 60%, white), 
    color-mix(in srgb, var(--theme-color) 30%, rgba(255, 255, 255, 0.1))
  );
  border: 1px solid rgba(255, 255, 255, 0.18);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.2),
    0 0 30px var(--theme-glow);
  animation: center-glow 3s ease-in-out infinite;
}

.center-ball .icon {
  color: white;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
}

@keyframes center-glow {
  0%, 100% {
    box-shadow: 
      0 4px 20px rgba(0, 0, 0, 0.3),
      0 0 30px var(--theme-glow);
  }
  50% {
    box-shadow: 
      0 4px 25px rgba(0, 0, 0, 0.4),
      0 0 45px var(--theme-glow);
  }
}

@keyframes light-wave {
  0% {
    transform: translate(-50%, -50%) scale(0);
    opacity: 1;
  }
  100% {
    transform: translate(-50%, -50%) scale(3);
    opacity: 0;
  }
}

.no-device-hint {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  color: rgba(255, 255, 255, 0.4);
  font-size: 11px;
  text-align: center;
  margin-top: 70px;
  line-height: 1.6;
}
</style>
