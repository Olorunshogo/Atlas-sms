<script setup lang="ts">
import { CheckCircle2, AlertTriangle, XCircle, Info, X } from 'lucide-vue-next';
import { cn } from '../utils';
import { toastVariants } from './toast';
import type { ToastEntry } from './toast-types';

const props = defineProps<{ entry: ToastEntry }>();

const emit = defineEmits<{
  dismiss: [id: string];
}>();

const ICONS = {
  default: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  destructive: XCircle,
} as const;
</script>

<template>
  <div
    role="status"
    :aria-live="entry.variant === 'destructive' ? 'assertive' : 'polite'"
    :class="cn(toastVariants({ variant: entry.variant }))"
  >
    <component
      :is="ICONS[entry.variant ?? 'default']"
      class="mt-0.5 size-5 shrink-0"
    />
    <div class="flex flex-1 flex-col gap-0.5">
      <p class="text-sm font-semibold">{{ entry.title }}</p>
      <p v-if="entry.description" class="text-sm opacity-90">
        {{ entry.description }}
      </p>
    </div>
    <button
      type="button"
      class="shrink-0 rounded-full p-1 opacity-70 transition-opacity hover:opacity-100"
      @click="emit('dismiss', props.entry.id)"
    >
      <X class="size-4" />
      <span class="sr-only">Dismiss</span>
    </button>
  </div>
</template>
