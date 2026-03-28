<script setup>
import { computed } from "vue";
import { 
  Speaker, 
  Headphones, 
  Monitor, 
  Bluetooth,
  Volume2 
} from "lucide-vue-next";

const props = defineProps({
  device: {
    type: Object,
    required: true
  },
  isActive: {
    type: Boolean,
    default: false
  },
  position: {
    type: Object,
    default: () => ({ x: 0, y: 0 })
  },
  advancedMaterial: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(["click"]);

const deviceIcon = computed(() => {
  const deviceType = props.device.type || props.device.device_type;
  switch (deviceType) {
    case "speakers": return Speaker;
    case "headphones": return Headphones;
    case "hdmi": return Monitor;
    case "bluetooth": return Bluetooth;
    default: return Volume2;
  }
});

const truncatedName = computed(() => {
  const name = props.device.name;
  if (name.length <= 10) return name;
  return name.substring(0, 8) + "..";
});

function handleClick() {
  emit("click", props.device);
}
</script>

<template>
  <div
    class="device-ball"
    :class="[isActive ? 'snapped' : 'unsnapped', { 'advanced-material': advancedMaterial }]"
    :style="{
      left: `${position.x}px`,
      top: `${position.y}px`
    }"
    @click="handleClick"
  >
    <div class="ball-inner">
      <component :is="deviceIcon" :size="18" class="icon" />
    </div>
    <span class="name">{{ truncatedName }}</span>
  </div>
</template>

<style scoped>
.device-ball {
  position: absolute;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  z-index: 5;
  transition: transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  will-change: transform;
  transform: translateZ(0);
}

.device-ball:hover {
  transform: scale(1.1);
}

.device-ball:active {
  transform: scale(0.95);
}

.ball-inner {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.25s ease;
}

/* 深色模式 - 激活状态 */
.device-ball.snapped .ball-inner {
  background: linear-gradient(145deg, 
    var(--theme-color), 
    color-mix(in srgb, var(--theme-color) 65%, black)
  );
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.4),
    0 0 20px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

/* 深色模式 - 未激活状态 */
.device-ball.unsnapped .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.1), 
    rgba(255, 255, 255, 0.05)
  );
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 
    0 2px 8px rgba(0, 0, 0, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(12px) saturate(180%);
  -webkit-backdrop-filter: blur(12px) saturate(180%);
}

.device-ball.unsnapped:hover .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.15), 
    rgba(255, 255, 255, 0.08)
  );
  border-color: rgba(255, 255, 255, 0.2);
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.12);
}

/* 图标样式 */
.device-ball .icon {
  pointer-events: none;
  color: white;
  filter: drop-shadow(0 2px 3px rgba(0, 0, 0, 0.4));
}

/* 名称样式 */
.device-ball .name {
  position: absolute;
  bottom: -22px;
  font-size: 9px;
  color: var(--text-secondary);
  white-space: nowrap;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  pointer-events: none;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 激活动画 */
.device-ball.snapped {
  animation: glow-pulse 2s ease-in-out infinite;
}

/* 深色模式 - 高级材质激活状态 */
.device-ball.snapped.advanced-material .ball-inner {
  background: linear-gradient(145deg, 
    color-mix(in srgb, var(--theme-color) 65%, white), 
    color-mix(in srgb, var(--theme-color) 40%, rgba(255, 255, 255, 0.2))
  );
  border: 1px solid rgba(255, 255, 255, 0.3);
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 25px var(--theme-glow);
}

/* 深色模式 - 高级材质未激活状态 */
.device-ball.unsnapped.advanced-material .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.18), 
    rgba(255, 255, 255, 0.08)
  );
  border: 1px solid rgba(255, 255, 255, 0.18);
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
  box-shadow: 
    0 4px 20px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.device-ball.unsnapped.advanced-material:hover .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.25), 
    rgba(255, 255, 255, 0.12)
  );
  border-color: rgba(255, 255, 255, 0.3);
  box-shadow: 
    0 8px 28px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.3);
}

/* ========== 浅色模式 ========== */

/* 浅色模式 - 未激活状态 */
[data-theme="light"] .device-ball.unsnapped .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.95), 
    rgba(255, 255, 255, 0.85)
  );
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow: 
    0 2px 10px rgba(0, 0, 0, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 1);
}

[data-theme="light"] .device-ball.unsnapped:hover .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 1), 
    rgba(255, 255, 255, 0.95)
  );
  border-color: rgba(0, 0, 0, 0.1);
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 1);
}

/* 浅色模式 - 激活状态 */
[data-theme="light"] .device-ball.snapped .ball-inner {
  background: linear-gradient(145deg, 
    var(--theme-color), 
    color-mix(in srgb, var(--theme-color) 75%, white)
  );
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.15),
    0 0 20px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.3);
}

/* 浅色模式 - 高级材质未激活状态 */
[data-theme="light"] .device-ball.unsnapped.advanced-material .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 0.98), 
    rgba(255, 255, 255, 0.92)
  );
  border: 1px solid rgba(0, 0, 0, 0.05);
  box-shadow: 
    0 4px 20px rgba(0, 0, 0, 0.06),
    inset 0 2px 0 rgba(255, 255, 255, 1);
}

[data-theme="light"] .device-ball.unsnapped.advanced-material:hover .ball-inner {
  background: linear-gradient(145deg, 
    rgba(255, 255, 255, 1), 
    rgba(255, 255, 255, 0.98)
  );
  border-color: rgba(0, 0, 0, 0.08);
  box-shadow: 
    0 8px 28px rgba(0, 0, 0, 0.1),
    inset 0 2px 0 rgba(255, 255, 255, 1);
}

/* 浅色模式 - 高级材质激活状态 */
[data-theme="light"] .device-ball.snapped.advanced-material .ball-inner {
  background: linear-gradient(145deg, 
    color-mix(in srgb, var(--theme-color) 85%, white), 
    color-mix(in srgb, var(--theme-color) 65%, white)
  );
  border: 1px solid rgba(255, 255, 255, 0.6);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 25px var(--theme-glow);
}

/* 浅色模式 - 图标 */
[data-theme="light"] .device-ball .icon {
  color: white;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.25));
}

[data-theme="light"] .device-ball.unsnapped .icon {
  color: var(--theme-color);
  filter: drop-shadow(0 1px 2px rgba(255, 255, 255, 0.8));
}

/* 浅色模式 - 名称 */
[data-theme="light"] .device-ball .name {
  text-shadow: 0 1px 3px rgba(255, 255, 255, 0.9);
}

@keyframes glow-pulse {
  0%, 100% {
    filter: drop-shadow(0 0 8px var(--theme-glow));
  }
  50% {
    filter: drop-shadow(0 0 16px var(--theme-glow));
  }
}
</style>
