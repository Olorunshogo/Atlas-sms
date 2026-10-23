import type { Component } from 'vue';

/*
 * The engine knows no dialog names.
 * An app maps its own keys to lazy loaders, and triggerDialog only receives a key.
 */

export type DialogProps = Record<string, unknown>;

export type DialogLoader = () => Promise<{ default: Component }>;

export type DialogRegistry = Record<string, DialogLoader>;

export interface OpenDialog {
  key: string;
  props: DialogProps;
  id: number;
  closing: boolean;
}

export interface DialogContextValue {
  triggerDialog: (key: string, props?: DialogProps) => void;
  closeDialog: () => void;
  closeAllDialogs: () => void;
  setDialogLoading: (loading: boolean) => void;
  setDialogDismissible: (dismissible: boolean) => void;
  openDialogs: readonly OpenDialog[];
}
