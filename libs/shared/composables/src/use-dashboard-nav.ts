import type { Component } from 'vue';
import { useRoute } from 'vue-router';
import {
  LayoutDashboard,
  ShoppingCart,
  ClipboardList,
  Wallet,
  Bell,
  User,
  MessageCircle,
  HelpCircle,
  ClipboardPenLine,
  Trophy,
  Package,
  MapPin,
  Banknote,
  Calculator,
  Mail,
  Truck,
  Users,
  PhoneCall,
  History,
  PackageCheck,
  UserCog,
  BarChart3,
  Headset,
} from 'lucide-vue-next';
import { supportWhatsAppHref } from '@atlas/data';

/*
 * Ported from the reference source's React use-dashboard-nav.ts (useLocation -> useRoute,
 * lucide-react -> lucide-vue-next). THE NAV CONTENT BELOW IS 100%
 * SPECIFIC TO THAT SOURCE (buyer/agent/admin-dashboard groups: wallet, stock
 * requests, procurement targets, admin invites...) and has no Atlas SMS
 * equivalent - it is ported verbatim as data, not adapted. Atlas's own
 * dashboard shell (not built yet) needs its own nav groups entirely -
 * Overview / Inventory / Sales / Purchasing & Transfers / People & Access /
 * Finance & Reports / Settings & Audit, gated by role_grants scopes, not by
 * AdminTier/AdminDesk, which are auth concepts from that source that Atlas does not have.
 * Keep the generic engine below (NavItem/NavGroup shape, tier/desk
 * filtering, active-href matching) as the pattern; replace the three
 * NavGroup[] constants entirely before using this in Atlas.
 */

export type AdminTier = 'super' | 'sub';

export type AdminDesk = 'buyer' | 'agent' | 'hr' | 'tech';

export type AdminDomain =
  | 'buyer'
  | 'agent'
  | 'supply'
  | 'finance'
  | 'catalogue'
  | 'fulfilment'
  | 'support'
  | 'growth'
  | 'hr';

/*
 * `tiers` is which admin tiers may see this item; omitted means every tier.
 * `desks` is which sub desks may see this item; omitted means every desk.
 * Super always sees every item (desks are ignored for super).
 * `badge` marks a surface that is routed and readable but not yet built.
 */
export interface NavItem {
  label: string;
  href: string;
  icon: Component;
  tiers?: readonly AdminTier[];
  desks?: readonly AdminDesk[];
  domain?: AdminDomain;
  badge?: string;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

const buyerNavGroups: NavGroup[] = [
  {
    label: 'MAIN',
    items: [
      { label: 'Overview', icon: LayoutDashboard, href: '/buyer-dashboard' },
      {
        label: 'Shop / Catalog',
        icon: ShoppingCart,
        href: '/buyer-dashboard/shop',
      },
      {
        label: 'My Orders',
        icon: ClipboardList,
        href: '/buyer-dashboard/orders',
      },
    ],
  },
  {
    label: 'ACCOUNT',
    items: [
      {
        label: 'Wallet & Payment',
        icon: Wallet,
        href: '/buyer-dashboard/wallet',
      },
      {
        label: 'Notifications',
        icon: Bell,
        href: '/buyer-dashboard/notifications',
      },
      {
        label: 'Profile & Address',
        icon: User,
        href: '/buyer-dashboard/settings',
      },
    ],
  },
  {
    label: 'SUPPORT',
    items: [
      {
        label: 'WhatsApp Support',
        icon: MessageCircle,
        href: supportWhatsAppHref(),
      },
      { label: 'Help Center', icon: HelpCircle, href: '/buyer-dashboard/help' },
    ],
  },
];

const agentNavGroups: NavGroup[] = [
  {
    label: 'MAIN',
    items: [
      { label: 'Overview', icon: LayoutDashboard, href: '/agent-dashboard' },
      {
        label: 'Request Stock',
        icon: ShoppingCart,
        href: '/agent-dashboard/request-stock',
      },
      {
        label: 'Daily Report',
        icon: ClipboardPenLine,
        href: '/agent-dashboard/daily-report',
      },
      { label: 'Deliveries', icon: Truck, href: '/agent-dashboard/deliveries' },
      {
        label: 'Leader board',
        icon: Trophy,
        href: '/agent-dashboard/leaderboard',
      },
    ],
  },
  {
    label: 'ACCOUNT',
    items: [
      {
        label: 'Wallet & Payment',
        icon: Wallet,
        href: '/agent-dashboard/wallet',
      },
      {
        label: 'Notification',
        icon: Bell,
        href: '/agent-dashboard/notifications',
      },
      {
        label: 'Profile & Address',
        icon: User,
        href: '/agent-dashboard/settings',
      },
    ],
  },
  {
    label: 'SUPPORT',
    items: [
      {
        label: 'WhatsApp Support',
        icon: MessageCircle,
        href: supportWhatsAppHref(),
      },
      { label: 'Help Center', icon: HelpCircle, href: '/agent-dashboard/help' },
    ],
  },
];

const adminNavGroups: NavGroup[] = [
  {
    label: 'OVERVIEW',
    items: [
      { label: 'Overview', icon: LayoutDashboard, href: '/admin-dashboard' },
    ],
  },
  {
    label: 'AGENT DESK',
    items: [
      {
        label: 'Agents',
        icon: User,
        href: '/admin-dashboard/agents',
        desks: ['agent'],
        domain: 'agent',
      },
      {
        label: 'Stock Requests',
        icon: PackageCheck,
        href: '/admin-dashboard/stock-requests',
        desks: ['agent'],
        domain: 'agent',
      },
      {
        label: 'Directory',
        icon: Users,
        href: '/admin-dashboard/directory',
        desks: ['agent'],
        domain: 'agent',
      },
      {
        label: 'Outreach',
        icon: MapPin,
        href: '/admin-dashboard/outreach',
        desks: ['agent'],
        domain: 'growth',
      },
    ],
  },
  {
    label: 'BUYER DESK',
    items: [
      {
        label: 'Buyers',
        icon: ShoppingCart,
        href: '/admin-dashboard/buyers',
        desks: ['buyer'],
      },
      {
        label: 'Deliveries',
        icon: Truck,
        href: '/admin-dashboard/deliveries',
        desks: ['buyer'],
      },
      {
        label: 'Buyer Desk',
        icon: ClipboardList,
        href: '/admin-dashboard/buyer',
        desks: ['buyer'],
        domain: 'buyer',
      },
      {
        label: 'Order Tracking',
        icon: Truck,
        href: '/admin-dashboard/buyer/deliveries',
        desks: ['buyer'],
        domain: 'buyer',
      },
    ],
  },
  {
    label: 'CATALOGUE & PRICING',
    items: [
      {
        label: 'Products',
        icon: Package,
        href: '/admin-dashboard/products',
        tiers: ['super'],
        domain: 'catalogue',
      },
      {
        label: 'Pricing & Delivery',
        icon: Wallet,
        href: '/admin-dashboard/pricing',
        tiers: ['super'],
        domain: 'catalogue',
      },
    ],
  },
  {
    label: 'FINANCE',
    items: [
      {
        label: 'Payouts',
        icon: Banknote,
        href: '/admin-dashboard/payouts',
        tiers: ['super'],
        domain: 'finance',
      },
      {
        label: 'Procurement Targets',
        icon: Calculator,
        href: '/admin-dashboard/procurement-targets',
        tiers: ['super'],
        domain: 'supply',
      },
    ],
  },
  {
    label: 'PLATFORM & ADMIN',
    items: [
      {
        label: 'Admin Invites',
        icon: Mail,
        href: '/admin-dashboard/admin-invites',
        tiers: ['super'],
      },
      {
        label: 'Activity',
        icon: History,
        href: '/admin-dashboard/activity',
        tiers: ['super'],
      },
    ],
  },
  {
    label: 'ACCOUNT',
    items: [
      {
        label: 'Notifications',
        icon: Bell,
        href: '/admin-dashboard/notifications',
      },
      { label: 'Settings', icon: User, href: '/admin-dashboard/settings' },
    ],
  },
  {
    label: 'SUPPORT',
    items: [
      {
        label: 'WhatsApp Support',
        icon: MessageCircle,
        href: supportWhatsAppHref(),
      },
    ],
  },
  {
    label: 'OTHERS',
    items: [
      {
        label: 'Assisted Checkout',
        icon: PhoneCall,
        href: '/admin-dashboard/assisted-checkout',
        tiers: ['super'],
        badge: 'Soon',
      },
      {
        label: 'Team & Roles',
        icon: UserCog,
        href: '/admin-dashboard/team-roles',
        tiers: ['super'],
        badge: 'Soon',
      },
      {
        label: 'Analytics',
        icon: BarChart3,
        href: '/admin-dashboard/analytics',
        tiers: ['super'],
        badge: 'Soon',
      },
      {
        label: 'Support Console',
        icon: Headset,
        href: '/admin-dashboard/support-console',
        tiers: ['super'],
        badge: 'Soon',
      },
    ],
  },
];

/** Drops what this tier/desk may not see, and any group left empty by that. */
function navForAdmin(
  groups: NavGroup[],
  tier: AdminTier,
  desk: AdminDesk | null,
): NavGroup[] {
  return groups
    .map((group) => ({
      ...group,
      items: group.items.filter((item) => {
        if (item.tiers && !item.tiers.includes(tier)) return false;
        if (tier === 'super') return true;
        if (!item.desks) return true;
        return desk !== null && item.desks.includes(desk);
      }),
    }))
    .filter((group) => group.items.length > 0);
}

export interface UseDashboardNavResult {
  groups: NavGroup[];
  allItems: NavItem[];
  isActive: (href: string) => boolean;
  basePath: string;
  isBuyer: boolean;
  isAgent: boolean;
  isAdmin: boolean;
  isSubAdmin: boolean;
  adminDesk: AdminDesk | null;
}

/**
 * Tier and desk are supplied by the caller rather than read from an auth
 * context: this composable ships in a shared package and must not reach
 * into the consuming app's session.
 */
export function useDashboardNav(
  adminTier?: AdminTier | null,
  adminDesk?: AdminDesk | null,
): UseDashboardNavResult {
  const route = useRoute();
  const pathname = route.path;

  const isBuyer = pathname.startsWith('/buyer-dashboard');
  const isAgent = pathname.startsWith('/agent-dashboard');
  const isAdmin = pathname.startsWith('/admin-dashboard');

  /*
   * Defaults to the narrower tier: showing a sub-admin links they cannot
   * use is worse than hiding one from a super admin until the token loads.
   */
  const tier: AdminTier = adminTier === 'super' ? 'super' : 'sub';
  const desk: AdminDesk | null =
    adminDesk === 'buyer' ||
    adminDesk === 'agent' ||
    adminDesk === 'hr' ||
    adminDesk === 'tech'
      ? adminDesk
      : null;

  const isSubAdmin: boolean = isAdmin && tier === 'sub';

  const groups = isAdmin
    ? navForAdmin(adminNavGroups, tier, desk)
    : isAgent
      ? agentNavGroups
      : buyerNavGroups;

  const basePath = isAdmin
    ? '/admin-dashboard'
    : isAgent
      ? '/agent-dashboard'
      : '/buyer-dashboard';

  const allItems = groups.flatMap((g) => g.items);

  const activeHref =
    allItems.reduce<string | null>((best, item) => {
      if (!pathname.startsWith(item.href)) return best;
      if (best === null || item.href.length > best.length) return item.href;
      return best;
    }, null) ?? basePath;

  function isActive(href: string): boolean {
    return href === activeHref;
  }

  return {
    groups,
    allItems,
    isActive,
    basePath,
    isBuyer,
    isAgent,
    isAdmin,
    isSubAdmin,
    adminDesk: desk,
  };
}
