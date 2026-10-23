<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../../lib/utils';
import { badgeVariants, type BadgeVariants } from './badge';

interface BadgeProps {
  variant?: BadgeVariants['variant'];
  class?: HTMLAttributes['class'];
}

defineProps<BadgeProps>();
</script>

<template>
  <span :class="cn(badgeVariants({ variant }), $props.class)">
    <slot />
  </span>
</template>
