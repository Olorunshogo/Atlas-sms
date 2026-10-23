<script setup lang="ts">
import { X } from 'lucide-vue-next';
import { cn, formatFileSize } from '../utils';

/**
 * Supporting piece for the planned FileDropzone.vue (see
 * docs/backend/kyc-frontend-plan.md) - ported from the reference source's SelectedFileInfo.
 * Kept separate from FileDropzone itself since FileDropzone's web/native
 * split is real, not-yet-built work; this row is just presentational and
 * was cheap to bring over now.
 */

interface SelectedFileInfoProps {
  file?: File;
}

const props = defineProps<SelectedFileInfoProps>();
const emit = defineEmits<{ remove: [] }>();

const fileName = () =>
  props.file?.name ?? `Selected_file_${props.file?.lastModified ?? ''}`;
</script>

<template>
  <div
    v-if="file"
    :class="
      cn(
        'bg-surface-sunken flex w-full flex-col gap-2 rounded-md px-4 py-2 pr-2',
      )
    "
  >
    <div class="flex w-full items-center justify-between gap-3">
      <div class="flex-1">
        <p
          class="text-ink-primary font-outfit truncate text-sm leading-tight font-medium sm:text-base"
        >
          {{ fileName() }}
        </p>
        <small class="text-ink-secondary font-outfit text-xs">
          File size:
          <span class="font-medium">{{ formatFileSize(file.size) }}</span>
        </small>
      </div>

      <button
        type="button"
        class="text-ink-primary hover:bg-surface-sunken rounded-full p-2"
        @click="emit('remove')"
      >
        <X :size="16" />
      </button>
    </div>
  </div>
</template>
