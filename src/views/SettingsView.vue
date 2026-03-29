<script setup>
import { ref, onMounted, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { Speaker, Headphones, Monitor, Bluetooth, Volume2, ArrowLeft, ChevronDown, ChevronUp, RefreshCw } from "lucide-vue-next";

const props = defineProps({
  appVersion: {
    type: String,
    default: ""
  },
  initialDevices: {
    type: Array,
    default: () => []
  },
  initialDefaultDeviceId: {
    type: String,
    default: null
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

const emit = defineEmits(["close", "config-changed"]);

const devices = ref([]);
const selectedDeviceId = ref(null);
const savedDeviceId = ref(null);
const isDropdownOpen = ref(false);
const advancedMaterial = ref(false);
const isCheckingUpdate = ref(false);
const updateInfo = ref(null);
const isInitialized = ref(false);

const GITHUB_REPO = "CmzYa/sound_link";
const GITHUB_RELEASES_URL = `https://github.com/${GITHUB_REPO}/releases/latest`;

const selectedDevice = computed(() => {
  return devices.value.find(d => d.id === selectedDeviceId.value);
});

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
    savedDeviceId.value = config.default_device_id || null;
    selectedDeviceId.value = savedDeviceId.value;
    advancedMaterial.value = config.advanced_material || false;
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

async function saveConfig() {
  try {
    await invoke("set_config", { 
      deviceId: selectedDeviceId.value ?? "",
      advancedMaterial: advancedMaterial.value 
    });
    savedDeviceId.value = selectedDeviceId.value;
  } catch (e) {
    console.error("Failed to save config:", e);
  }
}

function getDeviceIcon(type) {
  switch (type) {
    case "speakers": return Speaker;
    case "headphones": return Headphones;
    case "hdmi": return Monitor;
    case "bluetooth": return Bluetooth;
    default: return Volume2;
  }
}

function selectDevice(deviceId) {
  selectedDeviceId.value = deviceId;
  isDropdownOpen.value = false;
}

function clearSelection() {
  selectedDeviceId.value = null;
  isDropdownOpen.value = false;
}

watch(selectedDeviceId, () => {
  if (!isInitialized.value) return;
  saveConfig();
  emit("config-changed");
});

watch(advancedMaterial, () => {
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
    
    console.log("当前版本:", currentVersion, "最新版本:", latestVersion);
    
    const comparison = compareVersions(currentVersion, latestVersion);
    console.log("版本比较结果:", comparison);
    
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
  // 使用 MainView 传入的初始数据，避免重复加载和视觉跳动
  if (props.initialDevices && props.initialDevices.length > 0) {
    devices.value = props.initialDevices;
  } else {
    await loadDevices();
  }
  
  savedDeviceId.value = props.initialDefaultDeviceId;
  selectedDeviceId.value = props.initialDefaultDeviceId;
  advancedMaterial.value = props.initialAdvancedMaterial;
  
  // 如果 MainView 已检测到更新，直接显示
  if (props.hasUpdate && props.latestVersion) {
    updateInfo.value = {
      hasUpdate: true,
      latestVersion: props.latestVersion,
      currentVersion: props.appVersion
    };
  }
  
  // 初始化完成后再启用 watch 监听
  isInitialized.value = true;
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
    
    <div class="setting-item">
      <div class="setting-label">默认设备</div>
      <p class="setting-hint">无连接时使用并在主窗口隐藏</p>
      
      <div class="dropdown">
        <div class="dropdown-trigger" @click="isDropdownOpen = !isDropdownOpen">
          <div class="dropdown-value">
            <template v-if="selectedDevice">
              <component :is="getDeviceIcon(selectedDevice.type)" :size="14" class="dropdown-icon" />
              <span>{{ selectedDevice.name }}</span>
            </template>
            <span v-else class="placeholder">未选择</span>
          </div>
          <ChevronDown v-if="!isDropdownOpen" :size="16" class="chevron" />
          <ChevronUp v-else :size="16" class="chevron" />
        </div>
        
        <div v-if="isDropdownOpen" class="dropdown-menu">
          <div
            class="dropdown-item"
            :class="{ selected: selectedDeviceId === null }"
            @click="clearSelection"
          >
            <span class="placeholder">未选择</span>
            <div v-if="selectedDeviceId === null" class="check">✓</div>
          </div>
          <div
            v-for="device in devices"
            :key="device.id"
            class="dropdown-item"
            :class="{ selected: selectedDeviceId === device.id }"
            @click="selectDevice(device.id)"
          >
            <div class="item-content">
              <component :is="getDeviceIcon(device.type)" :size="14" class="dropdown-icon" />
              <span>{{ device.name }}</span>
            </div>
            <div v-if="selectedDeviceId === device.id" class="check">✓</div>
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
    
    <div class="about-section">
      <div class="about-title">Sound Link</div>
      <div class="about-version">v{{ props.appVersion || '...' }}</div>
      <div class="about-desc">快速切换音频输出设备</div>
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
      <div v-if="updateInfo?.hasUpdate" class="update-info">
        发现新版本 v{{ updateInfo.latestVersion }}，点击按钮前往下载
      </div>
      <div class="about-license">GPL-3.0 License</div>
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
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
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
  margin: 0 0 8px 0;
  line-height: 1.4;
}

.dropdown {
  position: relative;
}

.dropdown-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.dropdown-trigger:hover {
  border-color: var(--theme-color);
}

.dropdown-value {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-color);
}

.dropdown-icon {
  color: var(--theme-color);
  flex-shrink: 0;
}

.placeholder {
  color: var(--text-secondary);
}

.chevron {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  max-height: 180px;
  overflow-y: auto;
  z-index: 10;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
}

.dropdown-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.dropdown-item:hover {
  background: color-mix(in srgb, var(--theme-color) 10%, transparent);
}

.dropdown-item.selected {
  background: color-mix(in srgb, var(--theme-color) 15%, transparent);
}

.item-content {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-color);
}

.check {
  color: var(--theme-color);
  font-size: 12px;
  font-weight: bold;
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
  margin-top: auto;
  padding-top: 16px;
  text-align: center;
  border-top: 1px solid var(--glass-border);
}

.about-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 4px;
}

.about-version {
  font-size: 11px;
  color: var(--theme-color);
  margin-bottom: 4px;
}

.about-desc {
  font-size: 10px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.about-links {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-bottom: 6px;
}

.about-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
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
  width: 14px;
  height: 14px;
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

.update-btn.has-update:hover {
  background: rgba(34, 197, 94, 0.2);
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.update-info {
  font-size: 10px;
  color: #22c55e;
  margin-top: 4px;
  margin-bottom: 4px;
}

.about-license {
  font-size: 10px;
  color: var(--text-secondary);
  opacity: 0.7;
}
</style>
