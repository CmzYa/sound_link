<script setup>
import { ref, onMounted, watch, computed, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { Speaker, Headphones, Monitor, Bluetooth, Volume2, ArrowLeft, RefreshCw } from "lucide-vue-next";

// 防抖定时器
const volumeDebounceTimers = ref({});
const delayDebounceTimers = ref({});
const DEBOUNCE_DELAY = 150;

const props = defineProps({
  appVersion: {
    type: String,
    default: ""
  },
  initialDevices: {
    type: Array,
    default: () => []
  },
  initialAdvancedMaterial: {
    type: Boolean,
    default: false
  },
  hasUpdate: {
    type: Boolean,
    default: false
  },
  latestVersion: {
    type: String,
    default: ""
  }
});

const emit = defineEmits(["close", "config-changed", "device-settings-changed"]);

const devices = ref([]);
const advancedMaterial = ref(false);
const autoStart = ref(false);
const isCheckingUpdate = ref(false);
const updateInfo = ref(null);
const isInitialized = ref(false);

// 所有设备的音量和延迟配置
const deviceVolumes = ref({});
const deviceDelays = ref({});

const GITHUB_REPO = "CmzYa/sound_link";
const GITHUB_RELEASES_URL = `https://github.com/${GITHUB_REPO}/releases/latest`;

// 过滤掉 Cable 设备
const availableDevices = computed(() => {
  return devices.value.filter(d => !d.name.toLowerCase().includes('cable'));
});

function getDeviceIcon(type) {
  switch (type) {
    case "speakers": return Speaker;
    case "headphones": return Headphones;
    case "hdmi": return Monitor;
    case "bluetooth": return Bluetooth;
    default: return Volume2;
  }
}

async function loadDevices() {
  try {
    devices.value = await invoke("get_audio_devices");
  } catch (e) {
    console.error("Failed to load devices:", e);
  }
}

async function loadConfig() {
  try {
    const config = await invoke("get_config");
    advancedMaterial.value = config.advanced_material || false;
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

async function saveConfig() {
  try {
    await invoke("set_config", { 
      advancedMaterial: advancedMaterial.value,
      autoStart: autoStart.value
    });
  } catch (e) {
    console.error("Failed to save config:", e);
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
      devices: availableDevices.value.map(d => ({
        id: d.id,
        name: d.name,
        volume: deviceVolumes.value[d.id] ?? 1.0,
        delay_ms: deviceDelays.value[d.id] ?? 0,
        enabled: true
      }))
    };
    await invoke("save_router_config", { config });
  } catch (e) {
    console.error("Failed to save device settings:", e);
  }
}

// 防抖更新设备音量
function updateDeviceVolume(deviceId, volume) {
  deviceVolumes.value = { ...deviceVolumes.value, [deviceId]: volume };
  
  if (volumeDebounceTimers.value[deviceId]) {
    clearTimeout(volumeDebounceTimers.value[deviceId]);
  }
  
  volumeDebounceTimers.value[deviceId] = setTimeout(async () => {
    await saveDeviceSettings();
    emit("device-settings-changed", { deviceId, volume, delayMs: deviceDelays.value[deviceId] });
    try {
      await invoke("set_router_device_volume", { deviceId, volume });
    } catch (e) {
    }
  }, DEBOUNCE_DELAY);
}

// 防抖更新设备延迟
function updateDeviceDelay(deviceId, delayMs) {
  deviceDelays.value = { ...deviceDelays.value, [deviceId]: delayMs };
  
  if (delayDebounceTimers.value[deviceId]) {
    clearTimeout(delayDebounceTimers.value[deviceId]);
  }
  
  delayDebounceTimers.value[deviceId] = setTimeout(async () => {
    await saveDeviceSettings();
    emit("device-settings-changed", { deviceId, volume: deviceVolumes.value[deviceId], delayMs });
    try {
      await invoke("set_router_device_delay", { deviceId, delayMs });
    } catch (e) {
    }
  }, DEBOUNCE_DELAY);
}

watch(advancedMaterial, () => {
  if (!isInitialized.value) return;
  saveConfig();
  emit("config-changed");
});

watch(autoStart, () => {
  if (!isInitialized.value) return;
  saveConfig();
  emit("config-changed");
});

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
  if (isCheckingUpdate.value) return;
  
  isCheckingUpdate.value = true;
  updateInfo.value = null;
  
  try {
    const response = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`);
    if (!response.ok) throw new Error("Failed to fetch release info");
    
    const release = await response.json();
    const latestVersion = release.tag_name.replace(/^v/, '');
    const currentVersion = props.appVersion || "0.0.0";
    
    const comparison = compareVersions(currentVersion, latestVersion);
    
    if (comparison > 0) {
      updateInfo.value = {
        hasUpdate: true,
        latestVersion,
        currentVersion,
        releaseUrl: release.html_url
      };
    } else {
      updateInfo.value = {
        hasUpdate: false,
        latestVersion,
        currentVersion
      };
    }
  } catch (e) {
    console.error("Failed to check for updates:", e);
    updateInfo.value = {
      hasUpdate: false,
      error: true
    };
  } finally {
    isCheckingUpdate.value = false;
  }
}

async function openReleasePage() {
  try {
    await open(GITHUB_RELEASES_URL);
  } catch (e) {
    console.error("Failed to open release page:", e);
  }
}

onMounted(async () => {
  if (props.initialDevices && props.initialDevices.length > 0) {
    devices.value = props.initialDevices;
  } else {
    await loadDevices();
  }
  
  advancedMaterial.value = props.initialAdvancedMaterial;
  
  try {
    autoStart.value = await invoke("get_auto_start_status");
  } catch (e) {
    console.error("Failed to load auto start status:", e);
  }
  
  if (props.hasUpdate && props.latestVersion) {
    updateInfo.value = {
      hasUpdate: true,
      latestVersion: props.latestVersion,
      currentVersion: props.appVersion
    };
  }
  
  await loadDeviceSettings();
  isInitialized.value = true;
});

// 清理所有防抖定时器
onUnmounted(() => {
  Object.values(volumeDebounceTimers.value).forEach(timer => {
    if (timer) clearTimeout(timer);
  });
  Object.values(delayDebounceTimers.value).forEach(timer => {
    if (timer) clearTimeout(timer);
  });
});
</script>

<template>
  <div class="settings-container">
    <div class="header">
      <button class="back-btn" @click="emit('close')">
        <ArrowLeft :size="16" />
      </button>
      <h2>设置</h2>
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
                  @input="updateDeviceVolume(device.id, $event.target.value / 100)"
                  class="slider"
                >
                <span class="row-value">{{ Math.round((deviceVolumes[device.id] ?? 1) * 100) }}%</span>
              </div>
              <div class="setting-row">
                <span class="row-label">延迟</span>
                <input 
                  type="range" 
                  min="0" 
                  max="500" 
                  :value="deviceDelays[device.id] ?? 0"
                  @input="updateDeviceDelay(device.id, parseInt($event.target.value))"
                  class="slider"
                >
                <span class="row-value">{{ deviceDelays[device.id] ?? 0 }}ms</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div class="setting-item">
        <div class="setting-label">高级材质</div>
        <p class="setting-hint">启用毛玻璃效果和更丰富的视觉样式</p>
        
        <div class="toggle-container">
          <div 
            class="toggle-switch" 
            :class="{ active: advancedMaterial }"
            @click="advancedMaterial = !advancedMaterial"
          >
            <div class="toggle-thumb"></div>
          </div>
          <span class="toggle-label">{{ advancedMaterial ? '已开启' : '已关闭' }}</span>
        </div>
      </div>
      
      <div class="setting-item">
        <div class="setting-label">开机自启动</div>
        <p class="setting-hint">系统启动时自动运行应用</p>
        
        <div class="toggle-container">
          <div 
            class="toggle-switch" 
            :class="{ active: autoStart }"
            @click="autoStart = !autoStart"
          >
            <div class="toggle-thumb"></div>
          </div>
          <span class="toggle-label">{{ autoStart ? '已开启' : '已关闭' }}</span>
        </div>
      </div>
    </div>
    
    <div class="about-section">
      <div class="about-title">Sound Link</div>
      <div class="about-version">v{{ props.appVersion || '...' }}</div>
      <div class="about-links">
        <a href="https://github.com/CmzYa/sound_link" class="about-link" target="_blank">
          <svg class="github-icon" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
          </svg>
          GitHub
        </a>
        <button 
          class="about-link update-btn" 
          :class="{ 'has-update': updateInfo?.hasUpdate }"
          @click="updateInfo?.hasUpdate ? openReleasePage() : checkForUpdate()"
          :disabled="isCheckingUpdate"
        >
          <RefreshCw :size="14" :class="{ 'spinning': isCheckingUpdate }" />
          <span v-if="isCheckingUpdate">检查中...</span>
          <span v-else-if="updateInfo?.hasUpdate">有新版本</span>
          <span v-else-if="updateInfo && !updateInfo.error">已是最新</span>
          <span v-else>检查更新</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  width: 280px;
  height: 280px;
  padding: 12px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: var(--text-color);
  display: flex;
  flex-direction: column;
}

.header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.back-btn {
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

.back-btn:hover {
  background: color-mix(in srgb, var(--glass-bg) 120%, var(--theme-color));
  color: var(--text-color);
  border-color: var(--theme-color);
}

h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
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

.toggle-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toggle-switch {
  width: 44px;
  height: 24px;
  background: color-mix(in srgb, var(--text-color) 10%, transparent);
  border-radius: 12px;
  cursor: pointer;
  position: relative;
  transition: all 0.3s ease;
  border: 1px solid var(--glass-border);
}

.toggle-switch.active {
  background: var(--theme-color);
  border-color: var(--theme-color);
}

.toggle-thumb {
  width: 18px;
  height: 18px;
  background: white;
  border-radius: 50%;
  position: absolute;
  top: 2px;
  left: 3px;
  transition: all 0.3s ease;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.toggle-switch.active .toggle-thumb {
  left: 22px;
}

.toggle-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.about-section {
  flex-shrink: 0;
  padding-top: 12px;
  text-align: center;
  border-top: 1px solid var(--glass-border);
}

.about-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 2px;
}

.about-version {
  font-size: 10px;
  color: var(--theme-color);
  margin-bottom: 6px;
}

.about-links {
  display: flex;
  justify-content: center;
  gap: 8px;
}

.about-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: var(--text-secondary);
  text-decoration: none;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.2s;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
}

.about-link:hover {
  color: var(--theme-color);
  border-color: var(--theme-color);
  background: color-mix(in srgb, var(--theme-color) 10%, transparent);
}

.github-icon {
  width: 12px;
  height: 12px;
}

.update-btn {
  cursor: pointer;
  border: none;
  font-family: inherit;
}

.update-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.update-btn.has-update {
  color: #22c55e;
  border-color: #22c55e;
  background: rgba(34, 197, 94, 0.1);
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
