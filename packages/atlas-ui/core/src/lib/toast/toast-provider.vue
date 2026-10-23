<script setup lang="ts">
import { provide, ref } from 'vue';
import Toast from './toast.vue';
import { ATLAS_TOAST_KEY } from './use-atlas-toast';
import type {
  AtlasToastContext,
  ToastEntry,
  ToastOptions,
} from './toast-types';

const DEFAULT_DURATION = 5000;

const toasts = ref<ToastEntry[]>([]);
const timers = new Map<string, ReturnType<typeof setTimeout>>();

function dismiss(id: string): void {
  toasts.value = toasts.value.filter((entry) => entry.id !== id);
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
}

function toast(options: ToastOptions): string {
  const id =
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  const entry: ToastEntry = { id, variant: 'default', ...options };
  toasts.value = [...toasts.value, entry];

  const duration = options.duration ?? DEFAULT_DURATION;
  if (duration > 0) {
    timers.set(
      id,
      setTimeout(() => dismiss(id), duration),
    );
  }

  return id;
}

const context: AtlasToastContext = { toast, dismiss };
provide(ATLAS_TOAST_KEY, context);
</script>

<template>
  <slot />

  <div
    class="pointer-events-none fixed inset-x-0 bottom-0 z-[100] flex flex-col items-center gap-2 p-4 sm:inset-x-auto sm:right-0 sm:items-end"
  >
    <TransitionGroup
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-2"
      leave-active-class="transition-all duration-150 ease-in"
      leave-to-class="opacity-0"
    >
      <Toast
        v-for="entry in toasts"
        :key="entry.id"
        :entry="entry"
        class="sm:max-w-sm"
        @dismiss="dismiss"
      />
    </TransitionGroup>
  </div>
</template>
