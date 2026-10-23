<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';
import { Primitive, type PrimitiveProps } from 'reka-ui';
import { cn } from '../utils';

interface SecondaryLinkProps extends PrimitiveProps {
  label: string;
  href?: string;
  icon?: Component;
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<SecondaryLinkProps>(), {
  as: 'a',
});
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :class="
      cn(
        'font-outfit border-border-soft text-ink-primary flex h-12 items-center justify-center gap-4 rounded-full border px-6 py-3 transition-all duration-300 hover:opacity-90 hover:shadow-md',
        props.class,
      )
    "
  >
    <slot>
      <span>{{ label }}</span>
      <component :is="icon" v-if="icon" class="h-5 w-5" />
    </slot>
  </Primitive>
</template>
