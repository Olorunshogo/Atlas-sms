<script setup lang="ts">
interface AuthPanelProps {
  title: string;
  description?: string;
}

defineProps<AuthPanelProps>();
</script>

<template>
  <div class="mx-auto flex w-full max-w-112 flex-col gap-6">
    <div class="flex flex-col gap-2">
      <h1 class="text-3xl font-medium">{{ title }}</h1>
      <p v-if="description" class="text-ink-secondary">{{ description }}</p>
    </div>
    <slot />
    <div
      v-if="$slots.footer"
      class="text-ink-secondary flex flex-col gap-2 text-sm"
    >
      <slot name="footer" />
    </div>
  </div>
</template>
