<script setup lang="ts">
interface StoreChipProps {
  label: string;
}

defineProps<StoreChipProps>();
</script>

<template>
  <span
    class="bg-brand-soft text-brand rounded-pill px-3 py-1 text-sm font-semibold"
  >
    {{ label }}
  </span>
</template>
