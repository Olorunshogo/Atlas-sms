<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../utils';

interface CardProps {
  class?: HTMLAttributes['class'];
}

defineProps<CardProps>();
</script>

<template>
  <section
    :class="
      cn(
        'border-border-soft bg-surface-raised rounded-card flex flex-col gap-4 border p-6',
        $props.class,
      )
    "
  >
    <slot />
  </section>
</template>
