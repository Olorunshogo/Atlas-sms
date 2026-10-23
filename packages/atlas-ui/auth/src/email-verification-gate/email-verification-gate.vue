<script setup lang="ts">
import { AuthPanel } from '../auth-panel';
import { EmailVerificationBanner } from '../email-verification-banner';
import { VerifyEmailForm } from '../verify-email-form';

interface EmailVerificationGateProps {
  verified: boolean;
  email: string;
}

defineProps<EmailVerificationGateProps>();

const emit = defineEmits<{ verified: [] }>();
</script>

<template>
  <slot v-if="verified" />
  <div
    v-else
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg flex w-full flex-col gap-8"
    data-test="verification-gate"
  >
    <EmailVerificationBanner :email="email" />
    <AuthPanel
      title="Verify your email"
      :description="`Enter the 6-digit code sent to ${email}. Everything opens as soon as it's confirmed.`"
    >
      <VerifyEmailForm :email="email" @verified="emit('verified')" />
    </AuthPanel>
  </div>
</template>
