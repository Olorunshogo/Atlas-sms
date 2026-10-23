import type { Ref } from 'vue';
import { playModalContentExit, playModalFieldsEnter } from '../motion/presets';

export interface UseModalOptions {
  /** While true, nothing closes the modal: not Escape, the backdrop, Cancel, or a finished save. */
  isBlocked?: () => boolean;
}

export interface UseModal {
  playEnter: () => void;
  requestClose: () => void;
}

/**
 * The one close path for a `v-model` modal, ported from Celeris's `useModalMotion`.
 * `requestClose` fades the content out before flipping `open`, so the panel's own exit plays after it.
 */
export function useModal(
  open: Ref<boolean>,
  body: Readonly<Ref<HTMLElement | null>>,
  options: UseModalOptions = {},
): UseModal {
  const isBlocked = options.isBlocked ?? ((): boolean => false);

  function playEnter(): void {
    if (body.value) {
      playModalFieldsEnter(body.value);
    }
  }

  function requestClose(): void {
    if (isBlocked() || !open.value) {
      return;
    }
    if (!body.value) {
      open.value = false;
      return;
    }
    playModalContentExit(body.value, () => {
      open.value = false;
    });
  }

  return { playEnter, requestClose };
}
