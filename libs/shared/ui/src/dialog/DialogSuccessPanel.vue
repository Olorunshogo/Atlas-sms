<script setup lang="ts">
defineProps<{
  title: string;
  message?: string;
}>();
</script>

<template>
  <div class="flex flex-col gap-2">
    <h2 class="text-ink-primary text-lg font-semibold">{{ title }}</h2>
    <p v-if="message" class="text-ink-secondary text-sm">{{ message }}</p>
  </div>
</template>
