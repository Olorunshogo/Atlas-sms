<script setup lang="ts">
import {
  DialogErrorBanner,
  DialogHeader,
  useDialog,
  useDialogSubmission,
} from '@atlas/ui';

const props = defineProps<{
  title: string;
  body: string;
  confirmLabel?: string;
  tone?: 'default' | 'danger';
  onConfirm?: () => void | Promise<void>;
}>();

const dialog = useDialog();
const submission = useDialogSubmission<void>();

async function confirm(): Promise<void> {
  await submission.run(async () => {
    if (props.onConfirm) {
      await props.onConfirm();
    }
  });
  if (submission.status.value === 'success') {
    dialog.closeDialog();
  }
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <DialogHeader
      :title="title"
      :description="body"
      @close="dialog.closeDialog()"
    />
    <DialogErrorBanner
      v-if="submission.error.value"
      :message="submission.error.value"
    />
    <div class="flex flex-col gap-3 sm:flex-row sm:justify-end">
      <button
        type="button"
        class="border-border-inked text-ink-primary border px-4 py-3 text-sm font-semibold"
        :disabled="submission.isSubmitting.value"
        @click="dialog.closeDialog()"
      >
        Cancel
      </button>
      <button
        type="button"
        class="text-ink-inverse px-4 py-3 text-sm font-semibold"
        :class="tone === 'danger' ? 'bg-danger' : 'bg-brand'"
        :disabled="submission.isSubmitting.value"
        @click="confirm"
      >
        {{
          submission.isSubmitting.value
            ? 'Working'
            : (confirmLabel ?? 'Confirm')
        }}
      </button>
    </div>
  </div>
</template>
