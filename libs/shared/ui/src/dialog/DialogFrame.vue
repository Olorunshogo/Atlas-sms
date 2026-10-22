<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { playDialogEnter, playDialogExit } from '../motion/presets';

const props = defineProps<{
  closing: boolean;
  dismissible: boolean;
  stackIndex: number;
  isTop: boolean;
}>();

const emit = defineEmits<{
  finished: [];
  dismiss: [];
}>();

const backdrop = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);
const playedExit = ref<boolean>(false);
const previouslyFocused = ref<HTMLElement | null>(null);
let enterContext: { revert: () => void } | null = null;

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusable(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE));
}

function onKeyDown(event: KeyboardEvent): void {
  if (!props.isTop || !panel.value) {
    return;
  }
  if (event.key !== 'Tab') {
    return;
  }
  const items = focusable(panel.value);
  if (items.length === 0) {
    event.preventDefault();
    panel.value.focus();
    return;
  }
  const first = items[0];
  const last = items[items.length - 1];
  if (!first || !last) {
    return;
  }
  const active = document.activeElement;
  if (event.shiftKey && active === first) {
    event.preventDefault();
    last.focus();
    return;
  }
  if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

function playExit(): void {
  if (playedExit.value) {
    return;
  }
  playedExit.value = true;
  if (!backdrop.value || !panel.value) {
    emit('finished');
    return;
  }
  playDialogExit(backdrop.value, panel.value, () => {
    emit('finished');
  });
}

onMounted(() => {
  const active = document.activeElement;
  previouslyFocused.value = active instanceof HTMLElement ? active : null;
  if (backdrop.value && panel.value) {
    enterContext = playDialogEnter(backdrop.value, panel.value);
    panel.value.focus();
  }
  window.addEventListener('keydown', onKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown);
  if (enterContext) {
    enterContext.revert();
  }
  if (previouslyFocused.value) {
    previouslyFocused.value.focus();
  }
});

watch(
  () => props.closing,
  (closing: boolean) => {
    if (closing) {
      playExit();
    }
  },
);
</script>

<template>
  <div
    ref="backdrop"
    class="bg-surface-inked/60 fixed inset-0 opacity-0 backdrop-blur-sm motion-reduce:opacity-100"
    :class="dismissible ? 'cursor-pointer' : ''"
    :style="{ zIndex: 40 + stackIndex * 10 }"
    aria-hidden="true"
    @click="isTop && dismissible ? emit('dismiss') : undefined"
  />
  <div
    class="lg:px-section-px pointer-events-none fixed inset-0 flex items-end justify-center lg:items-center lg:py-6"
    :style="{ zIndex: 50 + stackIndex * 10 }"
  >
    <div
      ref="panel"
      role="dialog"
      :aria-modal="isTop"
      tabindex="-1"
      class="border-border-soft bg-surface-canvas pointer-events-auto flex max-h-[calc(100dvh-3rem)] w-full flex-col gap-6 overflow-y-auto rounded-t-2xl border p-6 opacity-0 outline-none motion-reduce:opacity-100 lg:max-w-xl lg:rounded-2xl"
    >
      <slot />
    </div>
  </div>
</template>
