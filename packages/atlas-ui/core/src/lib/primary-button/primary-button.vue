<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';

interface PrimaryButtonProps {
  label: string;
  icon?: Component;
  class?: HTMLAttributes['class'];
}

defineProps<PrimaryButtonProps>();
</script>

<template>
  <button
    :class="[
      'bg-brand text-brand-ink hover:bg-brand-hover inline-flex cursor-pointer items-center justify-center gap-2 rounded-full px-6 py-3 font-medium shadow-md transition hover:shadow-lg',
      $props.class,
    ]"
  >
    <component :is="icon" v-if="icon" class="h-5 w-5" />
    {{ label }}
  </button>
</template>
