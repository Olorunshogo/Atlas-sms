<script setup lang="ts">
import { defineAsyncComponent, provide, ref, watch } from 'vue';
import {
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
} from 'reka-ui';
import { cn } from '../utils';
import DialogLoader from './dialog-loader.vue';
import { ATLAS_DIALOG_KEY } from './use-atlas-dialog';
import type {
  CloseOptions,
  DialogEntry,
  DialogRegistry,
  DialogSize,
  TriggerOptions,
} from './dialog-types';

/**
 * `lg` matches the width the reference source's engine used before per-dialog sizing
 * existed, so dialogs that don't pass `size` keep the same width.
 */
const DIALOG_WIDTH: Record<DialogSize, string> = {
  sm: 'min(90%, 480px)',
  md: 'min(90%, 640px)',
  lg: 'min(90%, 900px)',
};

const props = withDefaults(
  defineProps<{
    /** Modal registry, keyed by the string passed to `triggerDialog`. Empty by
     * default - Atlas has no built-in dialogs yet, unlike the reference source's
     * DEFAULT_REGISTRY (ROLE_CHANGE/CONFIRM_ROLE_CHANGE), which were
     * auth flows specific to that source. Pass your app's own registry here. */
    registry?: DialogRegistry;
    /** Pass `route.fullPath` (or similar) from the consuming app to auto-close
     * every dialog on navigation, mirroring the reference source's `useLocation()` effect.
     * Left optional rather than importing vue-router directly, since this is
     * a framework-agnostic UI package. */
    routeKey?: string;
  }>(),
  { registry: () => ({}) },
);

const dialogs = ref<DialogEntry[]>([]);
const loadingMessage = ref<string>();

function closeDialog(
  callback?: () => Promise<void> | void,
  options?: CloseOptions,
) {
  if (dialogs.value.length) {
    if (options?.closeParent) {
      dialogs.value = dialogs.value.map((dialog) => ({
        ...dialog,
        isOpen: false,
      }));
    } else {
      const targetId =
        options?.dialogId ?? dialogs.value[dialogs.value.length - 1]?.id;
      const targetIndex = dialogs.value.findIndex(({ id }) => id === targetId);

      if (targetIndex >= 0) {
        dialogs.value = dialogs.value.map((dialog, index) =>
          index >= targetIndex ? { ...dialog, isOpen: false } : dialog,
        );
      }
    }
  }

  void callback?.();
}

function triggerDialog(
  key: string | 'CLOSE_DIALOG',
  data?: Record<string, unknown>,
  options?: TriggerOptions,
) {
  if (key === 'CLOSE_DIALOG') {
    closeDialog(undefined, { closeParent: true });
    return;
  }

  const loader = props.registry[key];
  if (!loader) {
    throw new Error(
      `Dialog "${key}" is not in the registry - pass it via the "registry" prop on AtlasDialogProvider.`,
    );
  }

  const asyncComponent = defineAsyncComponent({
    loader,
    loadingComponent: DialogLoader,
  });

  const dialogId =
    globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  const entry: DialogEntry = {
    id: dialogId,
    component: asyncComponent,
    data: data ?? {},
    isOpen: true,
    allowBackgroundDismissal: options?.allowBackgroundDismissal ?? true,
    size: options?.size ?? 'lg',
  };

  dialogs.value = options?.keepParent ? [...dialogs.value, entry] : [entry];
}

/**
 * Two-phase close: isOpen flips false first (starts the leave transition
 * below), then the entry is actually removed once that transition finishes -
 * mirrors the reference source's onWillDismiss (isOpen: false) / onDidDismiss (removeDialog).
 */
function removeDialog(dialogId: string) {
  dialogs.value = dialogs.value.filter(({ id }) => id !== dialogId);
}

function presentLoading(message = 'Loading...') {
  loadingMessage.value = message;
}

function closeLoading() {
  loadingMessage.value = undefined;
}

if (props.routeKey !== undefined) {
  watch(
    () => props.routeKey,
    () => closeDialog(undefined, { closeParent: true }),
  );
}

provide(ATLAS_DIALOG_KEY, {
  closeDialog,
  triggerDialog,
  presentLoading,
  closeLoading,
});

function onUpdateOpen(open: boolean, entry: DialogEntry) {
  if (!open) closeDialog(undefined, { dialogId: entry.id });
}

/**
 * Only the top-of-stack dialog is ever open with backgroundDismissal disabled
 * meaningfully, but every entry carries its own flag - a modal opened with
 * allowBackgroundDismissal: false (e.g. a destructive confirm) ignores both
 * backdrop clicks and Escape, same as the reference source's per-dialog escape handling.
 */
function onEscapeKeyDown(event: Event, entry: DialogEntry) {
  if (!entry.allowBackgroundDismissal) event.preventDefault();
}

function onPointerDownOutside(event: Event, entry: DialogEntry) {
  if (!entry.allowBackgroundDismissal) event.preventDefault();
}
</script>

<template>
  <slot />

  <DialogRoot
    v-for="entry in dialogs"
    :key="entry.id"
    :open="entry.isOpen"
    @update:open="(open) => onUpdateOpen(open, entry)"
  >
    <DialogPortal :force-mount="true">
      <Transition
        enter-active-class="transition-opacity duration-200"
        enter-from-class="opacity-0"
        leave-active-class="transition-opacity duration-150"
        leave-to-class="opacity-0"
      >
        <DialogOverlay
          v-show="entry.isOpen"
          class="bg-surface-inked/60 fixed inset-0 z-40"
        />
      </Transition>

      <Transition
        enter-active-class="transition-all duration-200 ease-out"
        enter-from-class="scale-95 opacity-0"
        leave-active-class="transition-all duration-150 ease-in"
        leave-to-class="scale-95 opacity-0"
        @after-leave="removeDialog(entry.id)"
      >
        <DialogContent
          v-show="entry.isOpen"
          :class="
            cn(
              'fixed top-1/2 left-1/2 z-50 -translate-x-1/2 -translate-y-1/2',
              'bg-surface-canvas max-h-[90vh] w-full overflow-y-auto rounded-2xl p-4 shadow-lg sm:p-6',
            )
          "
          :style="{ width: DIALOG_WIDTH[entry.size] }"
          @escape-key-down="onEscapeKeyDown($event, entry)"
          @pointer-down-outside="onPointerDownOutside($event, entry)"
        >
          <div class="relative flex flex-col gap-6">
            <component :is="entry.component" v-bind="entry.data" />
            <DialogLoader
              v-if="loadingMessage"
              :message="loadingMessage"
              class="absolute inset-0 z-50"
            />
          </div>
        </DialogContent>
      </Transition>
    </DialogPortal>
  </DialogRoot>
</template>
