<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';

interface GhostButtonProps {
  label: string;
  icon?: Component;
  class?: HTMLAttributes['class'];
}

defineProps<GhostButtonProps>();
</script>

<template>
  <button
    :class="[
      'text-ink-primary hover:bg-surface-sunken inline-flex cursor-pointer items-center justify-center gap-2 rounded-full px-6 py-3 font-medium transition',
      $props.class,
    ]"
  >
    <component :is="icon" v-if="icon" class="h-5 w-5" />
    {{ label }}
  </button>
</template>
