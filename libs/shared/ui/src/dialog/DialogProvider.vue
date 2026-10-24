<script setup lang="ts">
import {
  defineAsyncComponent,
  onBeforeUnmount,
  provide,
  reactive,
  ref,
  watch,
} from 'vue';
import type { Component } from 'vue';
import { useRouter } from 'vue-router';
import type {
  DialogContextValue,
  DialogProps,
  DialogRegistry,
  OpenDialog,
} from './dialog-types';
import { DIALOG_KEY } from './use-dialog';
import { lockScroll, unlockScroll } from '../motion/smooth-scroll';
import DialogFrame from './DialogFrame.vue';

const props = defineProps<{
  registry: DialogRegistry;
}>();

const openDialogs = ref<OpenDialog[]>([]);
const isLoading = ref<boolean>(false);
const isDismissible = ref<boolean>(true);
const nextId = ref<number>(0);
const componentCache = new Map<string, Component>();
const router = useRouter();

function resolveDialog(key: string): Component | null {
  const cached = componentCache.get(key);
  if (cached) {
    return cached;
  }
  const loader = props.registry[key];
  if (!loader) {
    return null;
  }
  const component = defineAsyncComponent(loader);
  componentCache.set(key, component);
  return component;
}

function triggerDialog(key: string, dialogProps: DialogProps = {}): void {
  if (!props.registry[key]) {
    console.error(
      '[DialogProvider] No dialog registered for key "' +
        key +
        '". Add it to the registry.',
    );
    return;
  }
  nextId.value += 1;
  openDialogs.value = [
    ...openDialogs.value,
    { key, props: dialogProps, id: nextId.value, closing: false },
  ];
}

function closeDialog(): void {
  const stack = openDialogs.value;
  const top = stack[stack.length - 1];
  if (!top || top.closing) {
    return;
  }
  isLoading.value = false;
  isDismissible.value = true;
  openDialogs.value = stack.map((dialog) => {
    if (dialog.id !== top.id) {
      return dialog;
    }
    return { ...dialog, closing: true };
  });
}

function closeAllDialogs(): void {
  isLoading.value = false;
  isDismissible.value = true;
  openDialogs.value = [];
}

function setDialogLoading(loading: boolean): void {
  isLoading.value = loading;
}

function setDialogDismissible(dismissible: boolean): void {
  isDismissible.value = dismissible;
}

function finishDialog(id: number): void {
  openDialogs.value = openDialogs.value.filter((dialog) => dialog.id !== id);
}

function onEscape(event: KeyboardEvent): void {
  if (event.key !== 'Escape') {
    return;
  }
  if (
    openDialogs.value.length === 0 ||
    isLoading.value ||
    !isDismissible.value
  ) {
    return;
  }
  closeDialog();
}

const context = reactive<DialogContextValue>({
  triggerDialog,
  closeDialog,
  closeAllDialogs,
  setDialogLoading,
  setDialogDismissible,
  openDialogs: openDialogs.value,
});

watch(openDialogs, (stack: OpenDialog[]) => {
  context.openDialogs = stack;
});

provide(DIALOG_KEY, context);

const removeGuard = router.afterEach(() => {
  closeAllDialogs();
});

let scrollLocked = false;
watch(openDialogs, (stack: OpenDialog[]) => {
  if (stack.length > 0 && !scrollLocked) {
    lockScroll();
    scrollLocked = true;
  } else if (stack.length === 0 && scrollLocked) {
    unlockScroll();
    scrollLocked = false;
  }
});

onBeforeUnmount(() => {
  removeGuard();
  if (scrollLocked) {
    unlockScroll();
  }
  window.removeEventListener('keydown', onEscape);
});

watch(
  () => openDialogs.value.length,
  (length: number, previous: number | undefined) => {
    if (length > 0 && (previous ?? 0) === 0) {
      window.addEventListener('keydown', onEscape);
    }
    if (length === 0) {
      window.removeEventListener('keydown', onEscape);
    }
  },
);
</script>

<template>
  <slot />
  <DialogFrame
    v-for="(dialog, index) in openDialogs"
    :key="dialog.id"
    :closing="dialog.closing"
    :dismissible="isDismissible && !isLoading"
    :stack-index="index"
    :is-top="index === openDialogs.length - 1"
    @dismiss="closeDialog"
    @finished="finishDialog(dialog.id)"
  >
    <component :is="resolveDialog(dialog.key)" v-bind="dialog.props" />
  </DialogFrame>
  <div
    v-if="isLoading"
    class="bg-surface-inked/40 fixed inset-0 z-[80] flex items-center justify-center"
  >
    <p
      class="bg-surface-canvas text-ink-primary px-6 py-4 text-sm font-semibold"
    >
      Please wait
    </p>
  </div>
</template>
