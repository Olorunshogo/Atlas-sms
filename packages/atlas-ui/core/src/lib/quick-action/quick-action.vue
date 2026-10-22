<script setup lang="ts">
import type { Component } from 'vue';

interface QuickActionProps {
  label: string;
  icon: Component;
}

defineProps<QuickActionProps>();
</script>

<template>
  <button
    type="button"
    class="bg-brand-soft text-ink-primary rounded-card flex cursor-pointer items-center gap-3 px-4 py-4 text-left text-sm font-semibold"
  >
    <component :is="icon" class="text-brand size-5 shrink-0" />
    {{ label }}
  </button>
</template>
