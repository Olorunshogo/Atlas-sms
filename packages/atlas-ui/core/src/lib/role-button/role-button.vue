<script setup lang="ts">
import type { Component, HTMLAttributes } from 'vue';
import { cn } from '../utils';

interface RoleButtonProps {
  icon: Component;
  label: string;
  description?: string;
  image?: string;
  class?: HTMLAttributes['class'];
}

const props = defineProps<RoleButtonProps>();
</script>

<template>
  <button
    v-if="image"
    type="button"
    :class="
      cn(
        'bg-surface-sunken text-ink-primary hover:bg-surface-sunken/90 group flex w-full min-w-62.5 flex-1 cursor-pointer flex-col overflow-hidden rounded-xl transition-all duration-300 ease-in-out',
        'lg:max-w-95',
        props.class,
      )
    "
  >
    <div class="relative h-50 w-full overflow-hidden lg:h-60">
      <img
        :src="image"
        :alt="label"
        class="h-full w-full object-cover transition-transform duration-500 ease-in-out group-hover:scale-105"
      />
      <div
        class="absolute inset-0 bg-linear-to-t from-black/40 to-transparent"
      />
    </div>
    <div class="flex flex-1 flex-col items-start gap-2 p-6">
      <div class="flex items-center gap-3">
        <component :is="icon" :size="22" :stroke-width="2" />
        <span class="text-xl font-semibold lg:text-2xl">{{ label }}</span>
      </div>
      <p
        v-if="description"
        class="text-ink-secondary text-left text-sm font-normal lg:text-base"
      >
        {{ description }}
      </p>
    </div>
  </button>

  <button
    v-else
    type="button"
    :class="
      cn(
        'bg-surface-sunken text-ink-primary hover:bg-surface-sunken/90 flex w-full cursor-pointer items-center gap-4 rounded-xl p-5 transition-all duration-300 ease-in-out hover:shadow-md',
        props.class,
      )
    "
  >
    <div
      class="bg-ink-primary/10 flex h-12 w-12 shrink-0 items-center justify-center rounded-lg"
    >
      <component :is="icon" :size="22" :stroke-width="2" />
    </div>
    <div class="flex flex-col items-start gap-0.5">
      <span class="text-lg font-semibold">{{ label }}</span>
      <p
        v-if="description"
        class="text-ink-secondary text-left text-sm font-normal"
      >
        {{ description }}
      </p>
    </div>
  </button>
</template>
