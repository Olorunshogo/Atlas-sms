<script setup lang="ts">
const DOTS = [0, 1, 2];
const DOT_DELAY_S = 0.2;
</script>

<template>
  <div
    role="status"
    aria-live="polite"
    class="bg-surface-inked/30 fixed inset-0 z-100 flex flex-col items-center justify-center gap-4 backdrop-blur-sm"
  >
    <slot name="logo" />
    <div class="flex items-center gap-1.5" aria-hidden="true">
      <span
        v-for="dot in DOTS"
        :key="dot"
        class="atlas-loader-dot bg-surface-canvas size-2 rounded-full"
        :style="{ animationDelay: `${dot * DOT_DELAY_S}s` }"
      />
    </div>
    <span class="sr-only">Loading</span>
  </div>
</template>

<style scoped>
@keyframes atlas-loader-pulse {
  0%,
  100% {
    opacity: 0.4;
    transform: scale(0.85);
  }
  50% {
    opacity: 1;
    transform: scale(1);
  }
}

.atlas-loader-dot {
  animation: atlas-loader-pulse 1.2s ease-in-out infinite;
}
</style>
