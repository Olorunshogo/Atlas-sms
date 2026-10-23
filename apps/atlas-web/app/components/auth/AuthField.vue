<script setup lang="ts">
defineOptions({ inheritAttrs: false });

interface AuthFieldProps {
  label: string;
  error?: string;
}

defineProps<AuthFieldProps>();
</script>

<template>
  <label class="flex flex-col gap-2">
    <span class="text-ink-secondary text-sm font-medium">{{ label }}</span>
    <input
      v-bind="$attrs"
      class="border-border-soft bg-surface-canvas text-ink-primary focus:border-border-inked rounded-full border px-4 py-3 outline-none"
    />
    <span v-if="error" class="text-danger text-sm">{{ error }}</span>
  </label>
</template>
