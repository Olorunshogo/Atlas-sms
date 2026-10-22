<script setup lang="ts">
import { AlertTriangleIcon } from 'lucide-vue-next';

defineProps<{ message?: string }>();
</script>

<template>
  <p v-if="message" class="text-body-sm text-danger flex items-center gap-2">
    <AlertTriangleIcon class="h-4 w-4 shrink-0" />
    {{ message }}
  </p>
</template>
