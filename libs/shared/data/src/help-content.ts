import type { Component } from 'vue';
import {
  AlertCircle,
  CreditCard,
  Package,
  ShoppingCart,
  TrendingUp,
  Truck,
  UserCircle,
  Wallet,
} from 'lucide-vue-next';

/*
 * From the reference source - help-centre copy for a buyer/agent marketplace dashboard
 * (ordering, payments, delivery tracking, wallet). Heavy prune candidate:
 * none of that matches Atlas SMS's domain. The original 427-line file's
 * actual Q&A copy (how to place an order, track a delivery, use a wallet,
 * etc.) was deliberately NOT ported - it is prose about a different
 * product and would be actively misleading sitting in Atlas's codebase.
 * Only the type shape is kept here, with icons swapped from lucide-react to
 * lucide-vue-next, so a real Atlas help-centre can reuse the pattern.
 * The original's dangling `HelpGuideSection` import (from a
 * components/help/help-guide-dialog module that isn't part of this copied
 * tree) is replaced by the local placeholder type below.
 */

export type HelpCategory =
  | 'ordering'
  | 'payments'
  | 'delivery'
  | 'problems'
  | 'account';

export interface HelpGuideEntry {
  question: string;
  answer: string;
}

/** Placeholder for the original HelpGuideSection - see file comment. */
export interface HelpGuideSection {
  category: HelpCategory;
  icon: Component;
  title: string;
  entries: HelpGuideEntry[];
}

/*
 * Icons carried over from the original category set so the mapping is
 * obvious if Atlas writes its own entries; the entries array is empty
 * since the original copy wasn't ported (see file comment).
 */
export const HELP_CATEGORY_ICONS: Record<HelpCategory, Component> = {
  ordering: ShoppingCart,
  payments: CreditCard,
  delivery: Truck,
  problems: AlertCircle,
  account: UserCircle,
};

/* Unused re-exports kept only to document which icons the original also referenced. */
export const _UNUSED_HELP_ICONS = { Package, TrendingUp, Wallet };

export const HELP_GUIDE_SECTIONS: readonly HelpGuideSection[] = [];
