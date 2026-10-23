<script setup lang="ts">
import type { Component } from 'vue';

interface EmptyStateProps {
  title: string;
  description?: string;
  icon?: Component;
}

defineProps<EmptyStateProps>();
</script>

<template>
  <div class="flex flex-col items-center gap-2 py-8 text-center">
    <component :is="icon" v-if="icon" class="size-6" aria-hidden="true" />
    <p class="font-semibold">{{ title }}</p>
    <p v-if="description" class="text-sm">{{ description }}</p>
  </div>
</template>
