<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '../../lib/utils';

interface TextareaProps {
  class?: HTMLAttributes['class'];
}

defineProps<TextareaProps>();
const modelValue = defineModel<string>();
</script>

<template>
  <textarea v-model="modelValue" :class="cn('w-full', $props.class)" />
</template>
