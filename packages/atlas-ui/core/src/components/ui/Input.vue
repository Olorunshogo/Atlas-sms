<script setup lang="ts">
import type { HTMLAttributes, InputTypeHTMLAttribute } from 'vue';
import { cn } from '../../lib/utils';

interface InputProps {
  type?: InputTypeHTMLAttribute;
  class?: HTMLAttributes['class'];
}

defineProps<InputProps>();
const modelValue = defineModel<string | number>();
</script>

<template>
  <input
    v-model="modelValue"
    :type="type"
    :class="
      cn(
        'border-input bg-background flex h-9 w-full rounded-md border px-3 py-1 text-sm shadow-sm transition-colors',
        'placeholder:text-muted-foreground',
        'focus-visible:ring-ring focus-visible:ring-1 focus-visible:outline-none',
        'disabled:cursor-not-allowed disabled:opacity-50',
        $props.class,
      )
    "
  />
</template>
