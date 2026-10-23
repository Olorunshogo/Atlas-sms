<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue';
import DialogFrame from './DialogFrame.vue';
import DialogHeader from './DialogHeader.vue';
import { useModal } from './use-modal';

/**
 * A page-owned modal, opened with `v-model:open`. Every way out (Escape, backdrop, Close, the slot's `close`)
 * goes through `requestClose`, which does nothing while `blocked`. Reset form state on `after-leave`,
 * once the modal is fully gone.
 */
interface ModalProps {
  title: string;
  description?: string;
  blocked?: boolean;
}

const props = withDefaults(defineProps<ModalProps>(), {
  description: undefined,
  blocked: false,
});

const emit = defineEmits<{ 'after-leave': [] }>();

const open = defineModel<boolean>('open', { default: false });
const body = useTemplateRef<HTMLElement>('body');
const rendered = ref<boolean>(open.value);
const closing = ref<boolean>(false);
const frameKey = ref<number>(0);
let previousOverflow = '';

const { playEnter, requestClose } = useModal(open, body, {
  isBlocked: () => props.blocked,
});

function onEscape(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    requestClose();
  }
}

function lockPage(): void {
  previousOverflow = document.body.style.overflow;
  document.body.style.overflow = 'hidden';
  window.addEventListener('keydown', onEscape);
}

function unlockPage(): void {
  document.body.style.overflow = previousOverflow;
  window.removeEventListener('keydown', onEscape);
}

watch(
  open,
  async (isOpen: boolean) => {
    if (isOpen) {
      closing.value = false;
      if (!rendered.value) {
        rendered.value = true;
        lockPage();
      }
      await nextTick();
      playEnter();
      return;
    }
    if (rendered.value) {
      closing.value = true;
    }
  },
  { immediate: true },
);

async function onFinished(): Promise<void> {
  // Reopened while the exit was still playing: start a fresh frame instead of unmounting
  if (open.value) {
    closing.value = false;
    frameKey.value += 1;
    await nextTick();
    playEnter();
    return;
  }
  rendered.value = false;
  closing.value = false;
  unlockPage();
  emit('after-leave');
}

onBeforeUnmount(() => {
  if (rendered.value) {
    unlockPage();
  }
});

defineExpose({ requestClose });
</script>

<template>
  <Teleport to="body">
    <DialogFrame
      v-if="rendered"
      :key="frameKey"
      :closing="closing"
      :dismissible="!blocked"
      :stack-index="0"
      :is-top="true"
      @dismiss="requestClose"
      @finished="onFinished"
    >
      <DialogHeader
        :title="title"
        :description="description"
        :show-closer="!blocked"
        @close="requestClose"
      />
      <div ref="body">
        <slot :close="requestClose" />
      </div>
    </DialogFrame>
  </Teleport>
</template>
