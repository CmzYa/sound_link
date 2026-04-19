<script setup>
import { computed } from "vue";
import {
  Speaker,
  Headphones,
  Monitor,
  Bluetooth,
  Volume2,
  Loader2,
} from "lucide-vue-next";

const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
  isActive: {
    type: Boolean,
    default: false,
  },
  isLoading: {
    type: Boolean,
    default: false,
  },
  position: {
    type: Object,
    default: () => ({ x: 0, y: 0 }),
  },
  advancedMaterial: {
    type: Boolean,
    default: false,
  },
  isRouterMode: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(["click"]);

const deviceIcon = computed(() => {
  if (props.isLoading) return Loader2;
  const deviceType = props.device.type;
  switch (deviceType) {
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
});

const truncatedName = computed(() => {
  const name = props.device.name;
  if (name.length <= 10) return name;
  return name.substring(0, 8) + "..";
});

function handleClick() {
  if (!props.isLoading) {
    emit("click", props.device);
  }
}
</script>

<template>
  <div
    class="device-ball"
    :class="[
      isActive ? 'snapped' : 'unsnapped',
      {
        'advanced-material': advancedMaterial,
        'is-loading': isLoading,
        'router-mode': isRouterMode,
      },
    ]"
    :style="{
      left: `${position.x}px`,
      top: `${position.y}px`,
    }"
    @click="handleClick"
  >
    <component
      :is="deviceIcon"
      :size="18"
      class="icon"
      :class="{ spin: isLoading }"
    />
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
  align-items: center;
  justify-content: center;
  z-index: 50;
  overflow: visible;
  cursor: pointer;
  transition:
    background 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s ease;
}

.device-ball:hover:not(.is-loading) {
  transform: scale(1.1);
}

.device-ball:active:not(.is-loading) {
  transform: scale(0.95);
}

.device-ball.is-loading {
  cursor: wait;
}

.icon.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* 深色模式 - 激活状态 */
.device-ball.snapped {
  background: linear-gradient(
    145deg,
    var(--theme-color),
    color-mix(in srgb, var(--theme-color) 65%, black)
  );
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.4),
    0 0 12px var(--theme-glow),
    0 0 24px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

/* 深色模式 - 未激活状态 */
.device-ball.unsnapped {
  background: linear-gradient(
    145deg,
    rgba(60, 60, 70, 0.95),
    rgba(45, 45, 55, 0.95)
  );
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.device-ball.unsnapped:hover {
  background: linear-gradient(
    145deg,
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

/* 深色模式 - 高级材质激活状态 */
.device-ball.snapped.advanced-material {
  background: linear-gradient(
    145deg,
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
.device-ball.unsnapped.advanced-material {
  background: linear-gradient(
    145deg,
    rgba(70, 70, 80, 0.98),
    rgba(50, 50, 60, 0.98)
  );
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow:
    0 4px 20px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.device-ball.unsnapped.advanced-material:hover {
  background: linear-gradient(
    145deg,
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
[data-theme="light"] .device-ball.unsnapped {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.95),
    rgba(255, 255, 255, 0.85)
  );
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow:
    0 2px 10px rgba(0, 0, 0, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 1);
}

[data-theme="light"] .device-ball.unsnapped:hover {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 1),
    rgba(255, 255, 255, 0.95)
  );
  border-color: rgba(0, 0, 0, 0.1);
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 1);
}

/* 浅色模式 - 激活状态 */
[data-theme="light"] .device-ball.snapped {
  background: linear-gradient(
    145deg,
    var(--theme-color),
    color-mix(in srgb, var(--theme-color) 75%, white)
  );
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.15),
    0 0 12px var(--theme-glow),
    0 0 24px var(--theme-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.3);
}

/* 浅色模式 - 高级材质未激活状态 */
[data-theme="light"] .device-ball.unsnapped.advanced-material {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 0.98),
    rgba(255, 255, 255, 0.92)
  );
  border: 1px solid rgba(0, 0, 0, 0.05);
  box-shadow:
    0 4px 20px rgba(0, 0, 0, 0.06),
    inset 0 2px 0 rgba(255, 255, 255, 1);
}

[data-theme="light"] .device-ball.unsnapped.advanced-material:hover {
  background: linear-gradient(
    145deg,
    rgba(255, 255, 255, 1),
    rgba(255, 255, 255, 0.98)
  );
  border-color: rgba(0, 0, 0, 0.08);
  box-shadow:
    0 8px 28px rgba(0, 0, 0, 0.1),
    inset 0 2px 0 rgba(255, 255, 255, 1);
}

/* 浅色模式 - 高级材质激活状态 */
[data-theme="light"] .device-ball.snapped.advanced-material {
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--theme-color) 85%, white),
    color-mix(in srgb, var(--theme-color) 65%, white)
  );
  border: 1px solid rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(20px) saturate(200%);
  -webkit-backdrop-filter: blur(20px) saturate(200%);
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

/* 路由模式样式 */
.device-ball.router-mode.snapped {
  background: linear-gradient(
    145deg,
    var(--router-color),
    color-mix(in srgb, var(--router-color) 65%, black)
  );
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.4),
    0 0 12px var(--router-glow),
    0 0 24px var(--router-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

.device-ball.router-mode.unsnapped:hover {
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--router-color) 30%, transparent),
    color-mix(in srgb, var(--router-color) 15%, transparent)
  );
  border-color: color-mix(in srgb, var(--router-color) 40%, transparent);
}

/* 浅色模式 - 路由模式 */
[data-theme="light"] .device-ball.router-mode.snapped {
  background: linear-gradient(
    145deg,
    var(--router-color),
    color-mix(in srgb, var(--router-color) 75%, white)
  );
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.15),
    0 0 12px var(--router-glow),
    0 0 24px var(--router-glow),
    inset 0 1px 0 rgba(255, 255, 255, 0.3);
}

/* 深色模式 - 高级材质路由模式 */
.device-ball.router-mode.snapped.advanced-material {
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--router-color) 65%, white),
    color-mix(in srgb, var(--router-color) 40%, rgba(255, 255, 255, 0.2))
  );
  border: 1px solid rgba(255, 255, 255, 0.3);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.25),
    inset 0 2px 0 rgba(255, 255, 255, 0.35),
    0 0 25px var(--router-glow);
}

/* 浅色模式 - 高级材质路由模式 */
[data-theme="light"] .device-ball.router-mode.snapped.advanced-material {
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--router-color) 85%, white),
    color-mix(in srgb, var(--router-color) 65%, white)
  );
  border: 1px solid rgba(255, 255, 255, 0.6);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 2px 0 rgba(255, 255, 255, 0.6),
    0 0 25px var(--router-glow);
}

/* 浅色模式 - 路由模式未激活 hover */
[data-theme="light"] .device-ball.router-mode.unsnapped:hover {
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--router-color) 25%, white),
    color-mix(in srgb, var(--router-color) 12%, white)
  );
  border-color: color-mix(in srgb, var(--router-color) 35%, transparent);
}

/* 浅色模式 - 路由模式未激活图标 */
[data-theme="light"] .device-ball.router-mode.unsnapped .icon {
  color: var(--router-color);
}
</style>
