<script setup lang="ts">
import type { ButtonHTMLAttributes, HTMLAttributes } from 'vue';
import { cn } from '../../lib/utils';
import { buttonVariants, type ButtonVariants } from './button';

interface ButtonProps {
  variant?: ButtonVariants['variant'];
  size?: ButtonVariants['size'];
  type?: ButtonHTMLAttributes['type'];
  class?: HTMLAttributes['class'];
}

withDefaults(defineProps<ButtonProps>(), {
  type: 'button',
});
</script>

<template>
  <button
    :type="type"
    :class="cn(buttonVariants({ variant, size }), $props.class)"
  >
    <slot />
  </button>
</template>
