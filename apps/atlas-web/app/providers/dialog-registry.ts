import type { DialogRegistry } from '@atlas/ui';

export const DIALOG_REGISTRY: DialogRegistry = {
  CONFIRM: () => import('../components/dialogs/ConfirmDialog.vue'),
};
