<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../utils';

interface FormLabelProps {
  class?: HTMLAttributes['class'];
  disabled?: boolean;
  htmlFor?: string;
  id?: string;
}

const props = withDefaults(defineProps<FormLabelProps>(), {
  disabled: false,
});
</script>

<template>
  <label
    :for="htmlFor || id"
    :class="
      cn(
        'font-outfit text-ink-muted text-left text-base',
        disabled ? 'cursor-not-allowed opacity-90' : 'text-ink-muted',
        props.class,
      )
    "
  >
    <slot />
  </label>
</template>
