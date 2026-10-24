<script setup lang="ts">
import type Lenis from 'lenis';
// lenis/nuxt ships a runtime plugin with no .d.ts, so import lenis/vue directly (same as Celeris)
import { VueLenis } from 'lenis/vue';
import { motionTokens, syncSmoothScroll } from '@atlas/ui';
import { usePrefersReducedMotion } from '@atlas/composables';

const reducedMotion = usePrefersReducedMotion();
const lenisRef = useTemplateRef<{ lenis?: Lenis }>('lenisRef');
const lenisOptions = {
  autoRaf: false,
  duration: motionTokens.smoothScrollDuration,
};

watchEffect((onCleanup) => {
  const lenis = lenisRef.value?.lenis;
  if (!lenis) {
    return;
  }
  onCleanup(syncSmoothScroll(lenis));
});
</script>

<template>
  <VueLenis v-if="!reducedMotion" ref="lenisRef" root :options="lenisOptions" />
</template>
