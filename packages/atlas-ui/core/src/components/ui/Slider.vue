<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import {
  SliderRoot,
  SliderTrack,
  SliderRange,
  SliderThumb,
  type SliderRootProps,
} from 'reka-ui';
import { cn } from '../../lib/utils';

interface Props extends Omit<SliderRootProps, 'modelValue'> {
  class?: HTMLAttributes['class'];
}

const props = defineProps<Props>();
const modelValue = defineModel<number[]>();
</script>

<template>
  <SliderRoot
    v-model="modelValue"
    v-bind="props"
    :class="
      cn(
        'relative flex w-full touch-none items-center select-none',
        props.class,
      )
    "
  >
    <SliderTrack
      class="bg-primary/20 relative h-1.5 w-full grow overflow-hidden rounded-full"
    >
      <SliderRange class="bg-primary absolute h-full" />
    </SliderTrack>
    <SliderThumb
      v-for="(_, index) in modelValue"
      :key="index"
      class="border-primary/50 bg-background focus-visible:ring-ring block h-4 w-4 rounded-full border shadow transition-colors focus-visible:ring-1 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
    />
  </SliderRoot>
</template>
