<script setup>
import { ref, onMounted, watch, defineEmits, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Speaker, Headphones, Monitor, Bluetooth, Volume2, ArrowLeft, ChevronDown, ChevronUp } from "lucide-vue-next";

const emit = defineEmits(["close", "config-changed"]);

const devices = ref([]);
const selectedDeviceId = ref(null);
const savedDeviceId = ref(null);
const isDropdownOpen = ref(false);
const advancedMaterial = ref(false);

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
  saveConfig();
  emit("config-changed");
});

watch(advancedMaterial, () => {
  saveConfig();
  emit("config-changed");
});

onMounted(async () => {
  await loadDevices();
  await loadConfig();
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
  </div>
</template>

<style scoped>
.settings-container {
  width: 280px;
  height: 280px;
  padding: 12px;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: rgba(255, 255, 255, 0.9);
  overflow-y: auto;
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
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  backdrop-filter: blur(10px) saturate(180%);
  -webkit-backdrop-filter: blur(10px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.back-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  color: rgba(255, 255, 255, 0.9);
  border-color: rgba(255, 255, 255, 0.1);
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
  color: rgba(255, 255, 255, 0.5);
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
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  backdrop-filter: blur(10px) saturate(180%);
  -webkit-backdrop-filter: blur(10px) saturate(180%);
}

.dropdown-trigger:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.15);
}

.dropdown-value {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.9);
}

.dropdown-icon {
  color: var(--theme-color);
  flex-shrink: 0;
}

.placeholder {
  color: rgba(255, 255, 255, 0.4);
}

.chevron {
  color: rgba(255, 255, 255, 0.5);
  flex-shrink: 0;
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: rgba(28, 28, 32, 0.85);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  max-height: 180px;
  overflow-y: auto;
  z-index: 10;
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
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
  background: rgba(255, 255, 255, 0.08);
}

.dropdown-item.selected {
  background: rgba(0, 120, 212, 0.15);
}

.item-content {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.9);
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
  background: rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  cursor: pointer;
  position: relative;
  transition: all 0.3s ease;
  border: 1px solid rgba(255, 255, 255, 0.1);
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
  color: rgba(255, 255, 255, 0.6);
}
</style>
