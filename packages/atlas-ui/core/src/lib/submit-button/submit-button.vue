<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { Button } from '../../components/ui';
import { cn } from '../utils';

export type SubmitButtonColor = 'brand' | 'neutral' | 'danger';
export type SubmitButtonVariant = 'solid' | 'outline';

interface SubmitButtonProps {
  /**
   * No defaults on color/variant by design - every call site must pick both
   * explicitly, so a submit button's look is never left to a fallback.
   * Merges the reference source's DashSubmitButton (blue/black solid) and
   * DashOutlineButton (bordered) into one locked-down matrix.
   */
  color: SubmitButtonColor;
  variant: SubmitButtonVariant;
  loading?: boolean;
  loadingText?: string;
  disabled?: boolean;
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<SubmitButtonProps>(), {
  loading: false,
  loadingText: 'Submitting...',
  disabled: false,
});

// === Color x variant matrix
/**
 * The one source of truth for how a submit button can look. Every
 * combination maps to an Atlas semantic token, never a raw hex value.
 */
const MATRIX: Record<SubmitButtonVariant, Record<SubmitButtonColor, string>> = {
  solid: {
    brand: 'bg-brand text-ink-inverse hover:bg-brand-hover',
    neutral: 'bg-ink-primary text-ink-inverse hover:opacity-90',
    danger: 'bg-danger text-ink-inverse hover:opacity-90',
  },
  outline: {
    brand: 'border-2 border-brand text-brand bg-transparent hover:bg-brand/5',
    neutral:
      'border-2 border-ink-primary text-ink-primary bg-transparent hover:bg-ink-primary/5',
    danger:
      'border-2 border-danger text-danger bg-transparent hover:bg-danger/5',
  },
};
</script>

<template>
  <Button
    type="submit"
    :disabled="loading || disabled"
    :class="
      cn(
        'font-outfit h-16 w-full rounded-full text-lg font-medium transition-all duration-300 ease-in-out hover:cursor-pointer',
        MATRIX[variant][color],
        (loading || disabled) && 'cursor-not-allowed opacity-70',
        props.class,
      )
    "
  >
    <template v-if="loading">{{ loadingText }}</template>
    <slot v-else>Submit</slot>
  </Button>
</template>
