<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
  showCloser?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();
</script>

<template>
  <div class="flex items-start justify-between gap-4">
    <div class="flex flex-col gap-1">
      <h2 class="text-ink-primary text-lg font-semibold">{{ title }}</h2>
      <p v-if="description" class="text-ink-secondary text-sm">
        {{ description }}
      </p>
    </div>
    <button
      v-if="showCloser !== false"
      type="button"
      class="text-ink-secondary text-sm font-semibold"
      @click="emit('close')"
    >
      Close
    </button>
  </div>
</template>
