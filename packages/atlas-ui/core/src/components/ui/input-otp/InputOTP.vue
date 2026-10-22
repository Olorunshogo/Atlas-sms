<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { OTPInput, type OTPInputProps } from 'vue-input-otp';
import { cn } from '../../../lib/utils';

interface Props extends Omit<OTPInputProps, 'class' | 'containerClass'> {
  class?: HTMLAttributes['class'];
  containerClass?: HTMLAttributes['class'];
}

const props = defineProps<Props>();
const modelValue = defineModel<string>();
</script>

<template>
  <OTPInput
    v-model="modelValue"
    v-bind="props"
    :container-class="
      cn(
        'flex items-center gap-2 has-[:disabled]:opacity-50',
        props.containerClass,
      )
    "
    :class="cn('disabled:cursor-not-allowed', props.class)"
  >
    <template #default="slotProps">
      <slot v-bind="slotProps" />
    </template>
  </OTPInput>
</template>
