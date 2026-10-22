<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { XIcon } from 'lucide-vue-next';
import { cn } from '../utils';

const props = defineProps<{ class?: HTMLAttributes['class'] }>();
</script>

<template>
  <button
    type="button"
    :class="
      cn(
        'flex cursor-pointer items-center justify-center rounded-md p-4 transition-all duration-300 ease-in-out hover:opacity-96',
        props.class,
      )
    "
  >
    <XIcon :size="24" />
    <span class="sr-only">Toggle menu</span>
  </button>
</template>
