<script setup lang="ts">
import { computed, type HTMLAttributes } from 'vue';
import { useVueOTPContext } from 'vue-input-otp';
import { cn } from '../../../lib/utils';

const props = defineProps<{
  index: number;
  class?: HTMLAttributes['class'];
}>();

const context = useVueOTPContext();
const slot = computed(() => context?.value.slots[props.index]);
</script>

<template>
  <div
    :class="
      cn(
        'border-input relative flex h-9 w-9 items-center justify-center border-y border-r text-sm shadow-sm transition-all first:rounded-l-md first:border-l last:rounded-r-md',
        slot?.isActive && 'ring-ring z-10 ring-1',
        props.class,
      )
    "
  >
    {{ slot?.char }}
    <div
      v-if="slot?.hasFakeCaret"
      class="pointer-events-none absolute inset-0 flex items-center justify-center"
    >
      <div class="animate-caret-blink bg-foreground h-4 w-px duration-1000" />
    </div>
  </div>
</template>
