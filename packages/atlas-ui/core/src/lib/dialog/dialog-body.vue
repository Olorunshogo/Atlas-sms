<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../utils';

/**
 * Named DialogBody, not DialogContent, to avoid colliding with reka-ui's own
 * DialogContent primitive used inside dialog-provider.vue.
 */
const props = defineProps<{ class?: HTMLAttributes['class'] }>();
</script>

<template>
  <div
    :class="cn('font-outfit flex w-full min-w-fit flex-col gap-4', props.class)"
  >
    <slot />
  </div>
</template>
