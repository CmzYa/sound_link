<script setup>
import { ref, onMounted, computed, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Speaker,
  Headphones,
  Monitor,
  Bluetooth,
  Volume2,
  X,
} from "lucide-vue-next";

const volumeDebounceTimers = ref({});
const delayDebounceTimers = ref({});
const DEBOUNCE_DELAY = 150;

const devices = ref([]);
const isInitialized = ref(false);
const deviceVolumes = ref({});
const deviceDelays = ref({});

const availableDevices = computed(() => {
  return devices.value.filter((d) => !d.name.toLowerCase().includes("cable"));
});

function getDeviceIcon(type) {
  switch (type) {
    case "speakers":
      return Speaker;
    case "headphones":
      return Headphones;
    case "hdmi":
      return Monitor;
    case "bluetooth":
      return Bluetooth;
    default:
      return Volume2;
  }
}

async function loadDevices() {
  try {
    devices.value = await invoke("get_audio_devices");
  } catch (e) {
    console.error("Failed to load devices:", e);
  }
}

async function loadDeviceSettings() {
  try {
    const savedConfig = await invoke("get_saved_router_config");
    if (savedConfig && savedConfig.devices) {
      for (const device of savedConfig.devices) {
        deviceVolumes.value[device.id] = device.volume;
        deviceDelays.value[device.id] = device.delay_ms;
      }
    }
  } catch (e) {
    console.error("Failed to load device settings:", e);
  }
}

async function saveDeviceSettings() {
  try {
    const config = {
      devices: availableDevices.value.map((d) => ({
        id: d.id,
        name: d.name,
        volume: deviceVolumes.value[d.id] ?? 1.0,
        delay_ms: deviceDelays.value[d.id] ?? 0,
        enabled: true,
      })),
    };
    await invoke("save_router_config", { config });
  } catch (e) {
    console.error("Failed to save device settings:", e);
  }
}

function updateDeviceVolume(deviceId, volume) {
  deviceVolumes.value = { ...deviceVolumes.value, [deviceId]: volume };

  if (volumeDebounceTimers.value[deviceId]) {
    clearTimeout(volumeDebounceTimers.value[deviceId]);
  }

  volumeDebounceTimers.value[deviceId] = setTimeout(async () => {
    await saveDeviceSettings();
    try {
      await invoke("set_router_device_volume", { deviceId, volume });
    } catch (e) {}
  }, DEBOUNCE_DELAY);
}

function updateDeviceDelay(deviceId, delayMs) {
  deviceDelays.value = { ...deviceDelays.value, [deviceId]: delayMs };

  if (delayDebounceTimers.value[deviceId]) {
    clearTimeout(delayDebounceTimers.value[deviceId]);
  }

  delayDebounceTimers.value[deviceId] = setTimeout(async () => {
    await saveDeviceSettings();
    try {
      await invoke("set_router_device_delay", { deviceId, delayMs });
    } catch (e) {}
  }, DEBOUNCE_DELAY);
}

async function closeWindow() {
  const window = getCurrentWindow();
  await window.close();
}

function hexToRgba(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

async function setupTheme() {
  let systemAccentColor = null;

  try {
    systemAccentColor = await invoke("get_system_accent_color");
  } catch (e) {
    console.error("Failed to get system accent color:", e);
  }

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
  document.documentElement.style.setProperty(
    "--theme-glow",
    hexToRgba(themeColor, 0.4)
  );

  if (isDark) {
    document.documentElement.style.setProperty(
      "--glass-bg",
      "rgba(28, 28, 32, 0.75)"
    );
    document.documentElement.style.setProperty(
      "--glass-border",
      "rgba(255, 255, 255, 0.08)"
    );
    document.documentElement.style.setProperty(
      "--text-color",
      "rgba(255, 255, 255, 0.9)"
    );
    document.documentElement.style.setProperty(
      "--text-secondary",
      "rgba(255, 255, 255, 0.6)"
    );
  } else {
    document.documentElement.style.setProperty(
      "--glass-bg",
      "rgba(255, 255, 255, 0.75)"
    );
    document.documentElement.style.setProperty(
      "--glass-border",
      "rgba(0, 0, 0, 0.08)"
    );
    document.documentElement.style.setProperty(
      "--text-color",
      "rgba(0, 0, 0, 0.9)"
    );
    document.documentElement.style.setProperty(
      "--text-secondary",
      "rgba(0, 0, 0, 0.6)"
    );
  }

  document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
}

onMounted(async () => {
  await setupTheme();
  await loadDevices();
  await loadDeviceSettings();
  isInitialized.value = true;
});

onUnmounted(() => {
  Object.values(volumeDebounceTimers.value).forEach((timer) => {
    if (timer) clearTimeout(timer);
  });
  Object.values(delayDebounceTimers.value).forEach((timer) => {
    if (timer) clearTimeout(timer);
  });
});
</script>

<template>
  <div class="router-settings-container">
    <div class="header" data-tauri-drag-region>
      <h2 data-tauri-drag-region>路由设置</h2>
      <button class="close-btn" @click="closeWindow">
        <X :size="16" />
      </button>
    </div>

    <div class="settings-scroll">
      <div class="setting-item">
        <div class="setting-label">设备音量与延迟</div>
        <p class="setting-hint">设置广播时各设备的音量和延迟</p>

        <div class="device-list">
          <div
            v-for="device in availableDevices"
            :key="device.id"
            class="device-item"
          >
            <div class="device-header">
              <component :is="getDeviceIcon(device.type)" :size="14" />
              <span class="device-name">{{ device.name }}</span>
            </div>

            <div class="device-settings">
              <div class="setting-row">
                <span class="row-label">音量</span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  :value="(deviceVolumes[device.id] ?? 1) * 100"
                  @input="
                    updateDeviceVolume(device.id, $event.target.value / 100)
                  "
                  class="slider"
                />
                <span class="row-value"
                  >{{
                    Math.round((deviceVolumes[device.id] ?? 1) * 100)
                  }}%</span
                >
              </div>
              <div class="setting-row">
                <span class="row-label">延迟</span>
                <input
                  type="range"
                  min="0"
                  max="500"
                  :value="deviceDelays[device.id] ?? 0"
                  @input="
                    updateDeviceDelay(device.id, parseInt($event.target.value))
                  "
                  class="slider"
                />
                <span class="row-value"
                  >{{ deviceDelays[device.id] ?? 0 }}ms</span
                >
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.router-settings-container {
  width: 300px;
  height: 320px;
  padding: 12px;
  font-family:
    -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: var(--text-color);
  display: flex;
  flex-direction: column;
  background: var(--glass-bg);
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  flex-shrink: 0;
  user-select: none;
  cursor: default;
}

h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.close-btn {
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
  border: 1px solid var(--glass-border);
}

.close-btn:hover {
  background: color-mix(in srgb, var(--glass-bg) 120%, #ef4444);
  color: #ef4444;
  border-color: #ef4444;
}

.settings-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.setting-item {
  margin-bottom: 12px;
}

.setting-label {
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
}

.setting-hint {
  font-size: 11px;
  color: var(--text-secondary);
  margin: 0 0 6px 0;
  line-height: 1.4;
}

.device-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.device-item {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  overflow: hidden;
}

.device-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  color: var(--theme-color);
}

.device-header .device-name {
  flex: 1;
  font-size: 12px;
  color: var(--text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.device-settings {
  padding: 4px 10px 8px;
}

.setting-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.row-label {
  font-size: 10px;
  color: var(--text-secondary);
  width: 28px;
}

.slider {
  flex: 1;
  height: 3px;
  -webkit-appearance: none;
  background: var(--glass-border);
  border-radius: 2px;
  outline: none;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  background: var(--theme-color);
  border-radius: 50%;
  cursor: pointer;
}

.row-value {
  font-size: 10px;
  color: var(--text-secondary);
  width: 36px;
  text-align: right;
}
</style>
