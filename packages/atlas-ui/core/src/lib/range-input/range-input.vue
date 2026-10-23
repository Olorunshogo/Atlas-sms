<script setup lang="ts">
import { useId, type HTMLAttributes } from 'vue';
import { FormLabel } from '../form-label';
import { cn } from '../utils';
import type { PriceRange } from './types';

interface RangeInputProps {
  min?: number;
  max?: number;
  step?: number;
  label?: string;
  class?: HTMLAttributes['class'];
  disabled?: boolean;
}

const props = withDefaults(defineProps<RangeInputProps>(), {
  min: 700_000,
  max: 15_000_000,
  step: 50_000,
  label: 'Price Range',
  disabled: false,
});

const value = defineModel<PriceRange>({ required: true });

const id = useId();

function formatPrice(amount: number) {
  if (amount >= 1_000_000) {
    return `₦${(amount / 1_000_000).toFixed(1)}M`;
  }
  return `₦${amount.toLocaleString()}`;
}

function handleChange(e: Event) {
  const newMax = Number((e.target as HTMLInputElement).value);
  value.value = { ...value.value, max: Math.max(value.value.min, newMax) };
}
</script>

<template>
  <div :class="cn('font-outfit w-full', props.class)">
    <div class="border-input flex w-full flex-col gap-3 border-b lg:border-0">
      <div class="flex items-center justify-between">
        <FormLabel :id="id">{{ label }}</FormLabel>

        <span class="text-ink-primary font-medium lg:hidden">
          {{ formatPrice(value.min) }} &ndash; {{ formatPrice(value.max) }}
        </span>
      </div>

      <div class="relative flex flex-col gap-3">
        <input
          :id="id"
          type="range"
          :min="min"
          :max="max"
          :step="step"
          :value="value.max"
          :disabled="disabled"
          :class="
            cn(
              'bg-border-soft h-2 w-full cursor-pointer appearance-none rounded-full transition-all',
              'accent-(--ink-primary)',
              '[&::-webkit-slider-thumb]:appearance-none',
              '[&::-webkit-slider-thumb]:h-5',
              '[&::-webkit-slider-thumb]:w-5',
              '[&::-webkit-slider-thumb]:rounded-full',
              '[&::-webkit-slider-thumb]:bg-ink-primary',
              '[&::-webkit-slider-thumb]:cursor-pointer',
              '[&::-webkit-slider-thumb]:shadow-md',
              '[&::-webkit-slider-thumb]:border-2',
              '[&::-webkit-slider-thumb]:border-white',
              '[&::-moz-range-thumb]:h-5',
              '[&::-moz-range-thumb]:w-5',
              '[&::-moz-range-thumb]:rounded-full',
              '[&::-moz-range-thumb]:bg-ink-primary',
              '[&::-moz-range-thumb]:border-2',
              '[&::-moz-range-thumb]:border-white',
              '[&::-moz-range-thumb]:cursor-pointer',
              disabled && 'cursor-not-allowed opacity-60',
            )
          "
          @input="handleChange"
        />

        <div class="hidden items-center justify-center lg:flex">
          <span class="text-ink-primary font-medium">
            {{ formatPrice(value.min) }} &ndash; {{ formatPrice(value.max) }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
