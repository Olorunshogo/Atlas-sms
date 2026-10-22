<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';

interface SecondaryButtonProps {
  label: string;
  icon?: Component;
  class?: HTMLAttributes['class'];
}

defineProps<SecondaryButtonProps>();
</script>

<template>
  <button
    :class="[
      'bg-teal text-ink-inverse hover:bg-teal-hover inline-flex cursor-pointer items-center justify-center gap-2 rounded-full px-6 py-3 font-medium shadow-sm transition',
      $props.class,
    ]"
  >
    <component :is="icon" v-if="icon" class="h-5 w-5" />
    {{ label }}
  </button>
</template>
