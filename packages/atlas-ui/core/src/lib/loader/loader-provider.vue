<script setup lang="ts">
import { onBeforeUnmount, provide, ref } from 'vue';
import LoaderOverlay from './loader-overlay.vue';
import { ATLAS_LOADER_KEY } from './use-atlas-loader';
import type { AtlasLoaderContext } from './loader-types';

/*
 * A ref-counted showLoader/hideLoader pair, not a boolean: a boolean breaks the moment two callers overlap, since the first one's hideLoader would hide the overlay while the second caller's operation is still running.
 * The overlay waits SHOW_DELAY_MS before appearing and then stays for at least MIN_VISIBLE_MS, so a fast operation never flashes it in and out.
 */
const SHOW_DELAY_MS = 150;
const MIN_VISIBLE_MS = 300;

const visible = ref<boolean>(false);
let count = 0;
let showTimer: ReturnType<typeof setTimeout> | null = null;
let hideTimer: ReturnType<typeof setTimeout> | null = null;
let shownAt: number | null = null;

onBeforeUnmount(() => {
  if (showTimer) clearTimeout(showTimer);
  if (hideTimer) clearTimeout(hideTimer);
});

function showLoader(): void {
  count += 1;
  if (count > 1) return;

  if (hideTimer) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }

  showTimer = setTimeout(() => {
    showTimer = null;
    if (count > 0) {
      shownAt = Date.now();
      visible.value = true;
    }
  }, SHOW_DELAY_MS);
}

function hideLoader(): void {
  count = Math.max(0, count - 1);
  if (count > 0) return;

  if (showTimer) {
    clearTimeout(showTimer);
    showTimer = null;
    return;
  }

  if (shownAt === null) return;

  const remaining = MIN_VISIBLE_MS - (Date.now() - shownAt);
  if (remaining <= 0) {
    visible.value = false;
    shownAt = null;
    return;
  }

  hideTimer = setTimeout(() => {
    hideTimer = null;
    if (count === 0) {
      visible.value = false;
      shownAt = null;
    }
  }, remaining);
}

async function withLoader<T>(action: () => Promise<T>): Promise<T> {
  showLoader();
  try {
    return await action();
  } finally {
    hideLoader();
  }
}

const context: AtlasLoaderContext = { showLoader, hideLoader, withLoader };
provide(ATLAS_LOADER_KEY, context);
</script>

<template>
  <slot />
  <LoaderOverlay v-if="visible">
    <template #logo>
      <slot name="logo" />
    </template>
  </LoaderOverlay>
</template>
