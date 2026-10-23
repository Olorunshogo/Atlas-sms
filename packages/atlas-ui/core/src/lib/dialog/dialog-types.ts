import type { Component } from 'vue';

export type DialogSize = 'sm' | 'md' | 'lg';

export type DialogAsyncLoader = () => Promise<{ default: Component }>;
export type DialogRegistry = Record<string, DialogAsyncLoader>;

export interface TriggerOptions {
  keepParent?: boolean;
  allowBackgroundDismissal?: boolean;
  size?: DialogSize;
}

export interface CloseOptions {
  closeParent?: boolean;
  dialogId?: string;
}

export interface DialogEntry {
  id: string;
  component: Component;
  data: Record<string, unknown>;
  isOpen: boolean;
  allowBackgroundDismissal: boolean;
  size: DialogSize;
}

export interface AtlasDialogContext {
  closeDialog: (
    callback?: () => Promise<void> | void,
    options?: CloseOptions,
  ) => void;
  triggerDialog: (
    key: string | 'CLOSE_DIALOG',
    data?: Record<string, unknown>,
    options?: TriggerOptions,
  ) => void;
  presentLoading: (message?: string) => void;
  closeLoading: () => void;
}
