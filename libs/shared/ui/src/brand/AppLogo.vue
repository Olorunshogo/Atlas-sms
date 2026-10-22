<script setup lang="ts">
import { ATLAS_CATCHPHRASE, ATLAS_SITE_NAME } from '@atlas/utils';

interface AppLogoProps {
  showCatchphrase?: boolean;
}

withDefaults(defineProps<AppLogoProps>(), {
  showCatchphrase: false,
});
</script>

<template>
  <span class="inline-flex flex-col gap-1">
    <span
      class="font-outfit text-brand text-lg leading-none font-semibold tracking-tight"
    >
      {{ ATLAS_SITE_NAME }}
    </span>
    <span v-if="showCatchphrase" class="text-ink-secondary max-w-xs text-sm">
      {{ ATLAS_CATCHPHRASE }}
    </span>
  </span>
</template>
