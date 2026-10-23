<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';
import { Primitive, type PrimitiveProps } from 'reka-ui';
import { cn } from '../utils';

interface PrimaryLinkProps extends PrimitiveProps {
  label: string;
  href?: string;
  icon?: Component;
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<PrimaryLinkProps>(), {
  as: 'a',
});
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :class="
      cn(
        'font-outfit bg-ink-primary text-ink-inverse flex h-12 w-fit cursor-pointer items-center justify-center gap-4 rounded-full px-6 py-3 text-lg shadow-md transition-all duration-300 hover:opacity-90 hover:shadow-lg',
        props.class,
      )
    "
  >
    <slot>
      <span>{{ label }}</span>
      <component :is="icon" v-if="icon" class="text-ink-inverse h-5 w-5" />
    </slot>
  </Primitive>
</template>
