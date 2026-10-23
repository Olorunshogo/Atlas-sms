<script setup lang="ts">
import type { Component } from 'vue';

interface StatCardProps {
  label: string;
  value: string | number;
  sub?: string;
  icon: Component;
}

defineProps<StatCardProps>();
</script>

<template>
  <div
    class="font-outfit border-border-soft bg-surface-raised rounded-card flex flex-col gap-3 border p-6 shadow-sm"
  >
    <div class="flex items-center justify-between">
      <span class="text-ink-secondary text-sm">{{ label }}</span>
      <span class="text-brand"><component :is="icon" /></span>
    </div>
    <span class="text-ink-primary tabular-figures text-3xl font-semibold">{{
      value
    }}</span>
    <span v-if="sub" class="text-ink-secondary text-xs">{{ sub }}</span>
  </div>
</template>
