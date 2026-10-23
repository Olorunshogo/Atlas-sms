export {
  MODAL_FIELD_SELECTOR,
  motionTokens,
  playDialogEnter,
  playDialogExit,
  playModalContentExit,
  playModalFieldsEnter,
  revealOnScroll,
  registerScrollTrigger,
} from './motion/presets';
export { useScrollReveal } from './motion/use-scroll-reveal';

export type {
  DialogContextValue,
  DialogProps,
  DialogRegistry,
  OpenDialog,
} from './dialog/dialog-types';
export { useDialog, useOptionalDialog } from './dialog/use-dialog';
export { useDialogSubmission } from './dialog/use-dialog-submission';
export type {
  DialogSubmissionStatus,
  UseDialogSubmissionResult,
} from './dialog/use-dialog-submission';
export { default as DialogProvider } from './dialog/DialogProvider.vue';
export { default as DialogHeader } from './dialog/DialogHeader.vue';
export { default as DialogErrorBanner } from './dialog/DialogErrorBanner.vue';
export { default as DialogSuccessPanel } from './dialog/DialogSuccessPanel.vue';
export { default as Modal } from './dialog/Modal.vue';
export { useModal } from './dialog/use-modal';
export type { UseModal, UseModalOptions } from './dialog/use-modal';

export type {
  BulkAction,
  RowAction,
  RowActionConfirm,
  TableColumn,
  TableSort,
  TableStateSnapshot,
} from './table/table-types';
export { useTableState } from './table/use-table-state';
export type {
  TableStateApi,
  UseTableStateOptions,
} from './table/use-table-state';
export { useTableUrlState } from './table/use-table-url-state';
export type { TableUrlState } from './table/use-table-url-state';
export { default as DataTable } from './table/DataTable.vue';
export { default as AppLogo } from './brand/AppLogo.vue';
