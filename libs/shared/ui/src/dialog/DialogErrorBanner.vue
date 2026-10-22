<script setup lang="ts">
defineProps<{
  message: string;
}>();
</script>

<template>
  <p
    class="border-danger bg-danger-soft text-danger border px-4 py-3 text-sm"
    role="alert"
  >
    {{ message }}
  </p>
</template>
