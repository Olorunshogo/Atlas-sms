<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../utils';

withDefaults(
  defineProps<{ message?: string; class?: HTMLAttributes['class'] }>(),
  {},
);
</script>

<template>
  <div
    :class="
      cn(
        'bg-surface-canvas flex flex-col items-center justify-center gap-3 py-6',
        $props.class,
      )
    "
  >
    <span
      class="border-border-soft border-t-ink-primary h-8 w-8 animate-spin rounded-full border-2"
      role="status"
      aria-label="Loading"
    />
    <p v-if="message" class="text-body-sm text-ink-secondary">{{ message }}</p>
  </div>
</template>
