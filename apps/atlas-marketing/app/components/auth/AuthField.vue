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
    <span class="text-sm font-medium text-ink-secondary">{{ label }}</span>
    <input
      v-bind="$attrs"
      class="rounded-full border border-border-soft bg-surface-canvas px-4 py-3 text-ink-primary outline-none focus:border-border-inked"
    />
    <span v-if="error" class="text-sm text-danger">{{ error }}</span>
  </label>
</template>
