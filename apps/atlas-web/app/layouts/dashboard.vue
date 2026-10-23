<script setup lang="ts">
import { DialogProvider } from '@atlas/ui';
import { EmailVerificationGate } from '@atlas/ui-auth';
import { DIALOG_REGISTRY } from '../providers/dialog-registry';

const session = useSession();

async function onVerified(): Promise<void> {
  await session.loadSession();
}
</script>

<template>
  <DialogProvider :registry="DIALOG_REGISTRY">
    <div
      class="bg-surface-canvas font-outfit text-ink-primary flex min-h-screen flex-col"
    >
      <SiteHeader />
      <div class="flex flex-1 flex-col">
        <EmailVerificationGate
          :verified="session.emailVerified.value"
          :email="session.profile.value?.user.email ?? ''"
          @verified="onVerified"
        >
          <slot />
        </EmailVerificationGate>
      </div>
      <SiteFooter />
    </div>
  </DialogProvider>
</template>
