<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cva } from 'class-variance-authority';
import { cn } from '../../lib/utils';

interface LabelProps {
  for?: string;
  class?: HTMLAttributes['class'];
}

const props = defineProps<LabelProps>();

const labelVariants = cva(
  'text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
);
</script>

<template>
  <label :for="props.for" :class="cn(labelVariants(), props.class)">
    <slot />
  </label>
</template>
