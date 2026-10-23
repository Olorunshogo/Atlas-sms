<script setup lang="ts">
interface EmailVerificationBannerProps {
  email: string;
}

defineProps<EmailVerificationBannerProps>();
</script>

<template>
  <div
    role="status"
    class="border-warning bg-warning-soft text-ink-primary flex flex-col gap-2 rounded-3xl border px-5 py-4 sm:flex-row sm:items-center sm:justify-between"
  >
    <p class="text-sm">
      <span class="font-semibold">Verify your email to continue.</span>
      We sent a code to {{ email }}. Your business data stays locked until it's
      confirmed.
    </p>
    <slot name="action" />
  </div>
</template>
