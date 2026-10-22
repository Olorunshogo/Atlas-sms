<script setup lang="ts">
import { DialogProvider } from '@atlas/ui';
import { DIALOG_REGISTRY } from '../providers/dialog-registry';
</script>

<template>
  <DialogProvider :registry="DIALOG_REGISTRY">
    <div
      class="bg-surface-canvas font-outfit text-ink-primary flex min-h-screen flex-col"
    >
      <SiteHeader />
      <div class="flex flex-1 flex-col">
        <slot />
      </div>
      <SiteFooter />
    </div>
  </DialogProvider>
</template>
