<script setup lang="ts">
import { XIcon } from 'lucide-vue-next';
import { DialogTitle } from 'reka-ui';
import { useAtlasDialog } from './use-atlas-dialog';

const props = withDefaults(
  defineProps<{ showCloser?: boolean; onClose?: () => void }>(),
  {
    showCloser: true,
  },
);
const { closeDialog } = useAtlasDialog();

function handleClose() {
  if (props.onClose) props.onClose();
  else closeDialog();
}
</script>

<template>
  <div
    class="flex w-full items-center justify-between gap-4 leading-none font-semibold"
  >
    <DialogTitle as-child>
      <div class="text-ink-primary flex flex-1 items-center gap-2 text-xl">
        <slot name="prefix" />
        <slot />
      </div>
    </DialogTitle>
    <div v-if="$slots.suffix || showCloser" class="flex items-center gap-2">
      <slot name="suffix" />
      <button
        v-if="showCloser"
        type="button"
        class="rounded-full p-1 transition-opacity hover:opacity-70"
        @click="handleClose"
      >
        <XIcon class="text-ink-primary size-5" />
        <span class="sr-only">Close</span>
      </button>
    </div>
  </div>
</template>
