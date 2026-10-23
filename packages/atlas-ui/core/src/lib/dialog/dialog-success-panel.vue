<script setup lang="ts">
import { CheckCircle2Icon } from 'lucide-vue-next';

defineProps<{ title: string; description?: string }>();
</script>

<template>
  <div class="flex flex-col items-center gap-2 py-4 text-center">
    <CheckCircle2Icon class="text-success h-8 w-8" />
    <p class="text-body text-ink-primary font-semibold">{{ title }}</p>
    <p v-if="description" class="text-body-sm text-ink-secondary">
      {{ description }}
    </p>
  </div>
</template>
