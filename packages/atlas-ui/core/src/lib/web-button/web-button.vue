<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';
import { cn } from '../utils';

interface WebButtonProps {
  icon?: Component;
  class?: HTMLAttributes['class'];
}

const props = defineProps<WebButtonProps>();
</script>

<template>
  <button
    :class="
      cn(
        'font-outfit bg-brand text-brand-ink hover:bg-brand-hover flex items-center justify-center gap-4 text-lg shadow-md transition-all duration-300 hover:cursor-pointer hover:shadow-lg disabled:cursor-not-allowed disabled:opacity-50',
        icon && !$slots.default
          ? 'h-12 w-12 rounded-full'
          : 'h-12 rounded-full px-6',
        props.class,
      )
    "
  >
    <component :is="icon" v-if="icon" class="text-brand-ink h-6 w-6" />
    <span v-if="$slots.default"><slot /></span>
  </button>
</template>
