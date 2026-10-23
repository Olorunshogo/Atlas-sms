import {
  AlertCircle,
  CreditCard,
  Package,
  ShoppingCart,
  TrendingUp,
  Truck,
  UserCircle,
  Wallet,
} from 'lucide-react';
import type { HelpGuideSection } from '../components/help/help-guide-dialog';

/*
 * Help centre copy for both dashboards, kept out of the route components so
 * the buyer and agent pages share one shell.
 *
 * Every answer here is checked against what the app actually does. If a flow
 * changes - a payment method, a status name, a button that no longer exists -
 * the copy has to change with it. Wrong help text at the payment step costs
 * more than no help text.
 */

// === Types

export type HelpCategory =
  | 'ordering'
  | 'payments'
  | 'delivery'
  | 'problems'
  | 'account';

export interface FaqItem {
  question: string;
  answer: string;
  category: HelpCategory;
}

export interface HelpCategoryTab {
  key: HelpCategory | 'all';
  label: string;
}

export interface HelpContent {
  guideTitle: string;
  guideSubtitle: string;
  categories: HelpCategoryTab[];
  faqs: FaqItem[];
  guideSections: HelpGuideSection[];
}

// === Buyer

const buyerFaqs: FaqItem[] = [
  {
    category: 'ordering',
    question: 'How do I place an order?',
    answer:
      'Open Shop from the sidebar, add what you need to your cart, then go to Checkout. Enter your state, LGA and delivery zone along with your address, choose Today or Tomorrow for delivery, then pay with your card or your wallet. Your order is confirmed as soon as the payment goes through.',
  },
  {
    category: 'ordering',
    question: 'Do I have to pay before my order is confirmed?',
    answer:
      'Yes. When you check out, your order is created with a Pending status and nothing is dispatched yet. It only moves to Paid once your card or wallet payment succeeds. We do not deliver against unpaid orders.',
  },
  {
    category: 'ordering',
    question: 'How is the delivery fee calculated?',
    answer:
      'The fee is based on the delivery zone you select and the items in your cart. It is quoted live at checkout as soon as you pick your zone, and the full total is shown in the summary before you pay. Nothing is added afterwards.',
  },
  {
    category: 'ordering',
    question: 'Can I leave instructions for the rider?',
    answer:
      'Yes. There is a Delivery Note field at checkout. Use it for landmarks, gate numbers, or a time that suits you best.',
  },
  {
    category: 'ordering',
    question: 'How do I quickly reorder something I bought before?',
    answer:
      'Open Shop from the sidebar. If you have ordered before, a Buy Again section appears at the top with the items you bought most recently, so you can add them straight to your cart.',
  },
  {
    category: 'payments',
    question: 'What payment methods can I use?',
    answer:
      'Two: your debit or credit card, processed securely by Paystack, or your wallet balance. We do not currently offer cash on delivery or manual bank transfer, so please do not hand cash to a rider.',
  },
  {
    category: 'payments',
    question: 'How do I pay with my wallet?',
    answer:
      'Choose Wallet at the checkout payment step. The order total is deducted from your available balance immediately and the order is marked Paid. You need enough available balance to cover the whole total.',
  },
  {
    category: 'payments',
    question: 'How do I add money to my wallet?',
    answer:
      'Go to Wallet in the sidebar and tap Add Funds. Enter the amount and you will be taken to Paystack to pay. Your balance updates as soon as the payment is confirmed.',
  },
  {
    category: 'payments',
    question: 'What if my wallet balance is not enough at checkout?',
    answer:
      'Checkout will tell you how much you have and how much is needed. Either top up in the Wallet section first, or switch the payment method to card and pay the difference that way.',
  },
  {
    category: 'payments',
    question: 'Are my card details safe?',
    answer:
      "Yes. Card payments happen on Paystack's own secure page, not on ours. The operator never sees or stores your card number, CVV or PIN.",
  },
  {
    category: 'payments',
    question: 'My card payment failed, or I closed the payment page. What now?',
    answer:
      'Nothing is taken from your account and your order simply stays Pending and unpaid. You can go back to checkout and try again. Orders that stay unpaid are cancelled automatically so they do not sit on your list.',
  },
  {
    category: 'payments',
    question: 'I was debited but my order still shows Pending.',
    answer:
      'Confirmation from Paystack occasionally arrives a little after the debit. Refresh My Orders in a few minutes and it should move to Paid. If it has not updated shortly after, contact support with your order number and we will trace the payment.',
  },
  {
    category: 'delivery',
    question: 'When will my order arrive?',
    answer:
      'You choose the window at checkout. Today is for orders placed before 12pm. Tomorrow is delivered between 9am and 5pm. You can follow the exact status at any time from My Orders.',
  },
  {
    category: 'delivery',
    question: 'Which areas do you deliver to?',
    answer:
      'We deliver across Kaduna by delivery zone. At checkout, pick your state and LGA and the zones we serve in that area will appear. If no zone shows up for your LGA, we do not cover it yet.',
  },
  {
    category: 'delivery',
    question: 'How do I track my order?',
    answer:
      'Go to My Orders in the sidebar. Use the tabs to filter by All, Active, Pending, Confirmed or Cancelled, and tap any order to see its full details.',
  },
  {
    category: 'delivery',
    question: 'Can I change my delivery address after ordering?',
    answer:
      'There is no self-service way to edit an address once the order is placed. Contact support as soon as you can with your order number and the correct address. We can only change it before the order goes out for delivery.',
  },
  {
    category: 'delivery',
    question: 'Do I have to be there to receive the delivery?',
    answer:
      'Someone needs to be available at the address to receive the items and confirm the delivery. If a neighbour or security post should take it instead, say so in the Delivery Note at checkout.',
  },
  {
    category: 'problems',
    question: 'What do the order statuses mean?',
    answer:
      'Pending means the order exists but has not been paid for yet. Paid means your payment is confirmed and we are preparing it. On the way means it has been dispatched. Delivered means it reached you. Cancelled means the order was stopped, either by you or because it was never paid for.',
  },
  {
    category: 'problems',
    question: 'Can I cancel my order?',
    answer:
      'You can cancel while the order is still Pending, meaning before it has been paid for. Open the order in My Orders, tap Cancel order and give a short reason. Once an order is paid it can no longer be cancelled from the app, so contact support to ask for a refund instead.',
  },
  {
    category: 'problems',
    question: 'How do refunds work?',
    answer:
      'Approved refunds are credited back to your wallet rather than your bank account. The balance is available immediately and you can spend it on your next order.',
  },
  {
    category: 'problems',
    question: 'I received the wrong item, or something is missing.',
    answer:
      'Contact support the same day with your order number and a clear photo of what you received. Getting in touch before the rider leaves makes it much easier to fix on the spot.',
  },
  {
    category: 'problems',
    question: 'My items arrived damaged or spoiled.',
    answer:
      'Take a photo before you put anything away, then contact support with your order number. If you can, check the items while the rider is still there so the problem can be confirmed straight away.',
  },
  {
    category: 'account',
    question: 'How do I log in?',
    answer:
      'With the email address and password you registered with. Logging in with a phone number is not supported, so use your email even if we also have your number on file.',
  },
  {
    category: 'account',
    question: 'How do I change my name, phone number or password?',
    answer:
      'Go to Settings in the sidebar. You can update your name and phone number there, and there is a separate section for changing your password. Your email address cannot be changed from the app.',
  },
  {
    category: 'account',
    question: 'Can I turn order notifications on or off?',
    answer:
      'Yes. In Settings you can switch email and SMS notifications on or off independently. Your order history is always available in My Orders regardless of those settings.',
  },
];

const buyerGuideSections: HelpGuideSection[] = [
  {
    icon: UserCircle,
    title: 'Getting Started',
    steps: [
      'Log in with the email address and password you registered with.',
      'Open Settings and confirm your name and phone number are correct, so we can reach you about a delivery.',
      'Use the sidebar to move between Shop, My Orders, Wallet and Settings.',
    ],
  },
  {
    icon: ShoppingCart,
    title: 'Browsing and Adding to Cart',
    steps: [
      'Open Shop from the sidebar to see everything currently available.',
      'If you have ordered before, check the Buy Again row at the top for your recent items.',
      'Tap a product to see its price and unit, choose a quantity, and add it to your cart.',
      'Open the cart to review your items before you head to checkout.',
    ],
  },
  {
    icon: Package,
    title: 'Checkout and Delivery Details',
    steps: [
      'Select your state and LGA, then pick the delivery zone that covers your area.',
      'Type the full delivery address, including landmarks that help a rider find you.',
      'Choose Today, for orders placed before 12pm, or Tomorrow between 9am and 5pm.',
      'Add a delivery note if there is anything the rider should know.',
      'Check the summary: the delivery fee is quoted live from your zone and the total shown is what you pay.',
    ],
  },
  {
    icon: CreditCard,
    title: 'Paying for Your Order',
    steps: [
      'Pick your method: card, handled securely by Paystack, or your wallet balance.',
      'Paying by card takes you to Paystack and back again once the payment succeeds.',
      'Paying by wallet deducts the total instantly, so make sure your available balance covers it.',
      'To load your wallet, go to Wallet in the sidebar, tap Add Funds and pay through Paystack.',
      'There is no cash on delivery and no manual bank transfer. Never hand cash to a rider.',
    ],
  },
  {
    icon: Truck,
    title: 'Following Your Order',
    steps: [
      'Pending: the order is created but not yet paid for, so nothing has been dispatched.',
      'Paid: your payment is confirmed and we are preparing the order.',
      'On the way: a rider has your order and is heading to your address.',
      'Delivered: the order reached you and is complete.',
      'Cancelled: the order was stopped, either by you or because it was never paid for.',
    ],
  },
  {
    icon: AlertCircle,
    title: 'Cancellations, Refunds and Problems',
    steps: [
      'You can cancel an order yourself while it is still Pending. Open it in My Orders, tap Cancel order and give a reason.',
      'Once an order is paid, cancelling in the app is no longer possible. Contact support to request a refund.',
      'Approved refunds are credited to your wallet and can be spent on your next order.',
      'For a wrong, missing or damaged item, send us your order number and a photo the same day.',
      'Check your items while the rider is still there whenever you can. It is the fastest way to get things put right.',
    ],
  },
];

export const buyerHelpContent: HelpContent = {
  guideTitle: 'Buyer Guide',
  guideSubtitle: 'How ordering, paying and delivery work',
  categories: [
    { key: 'all', label: 'All' },
    { key: 'ordering', label: 'Ordering' },
    { key: 'payments', label: 'Payments & Wallet' },
    { key: 'delivery', label: 'Delivery' },
    { key: 'problems', label: 'Problems & Refunds' },
    { key: 'account', label: 'Account' },
  ],
  faqs: buyerFaqs,
  guideSections: buyerGuideSections,
};

// === Agent

const agentFaqs: FaqItem[] = [
  {
    category: 'ordering',
    question: 'How do I request new stock?',
    answer:
      'Go to Request Stock from the sidebar, fill in the quantity you need, and submit. Your manager will approve the request and arrange delivery to your pickup point.',
  },
  {
    category: 'ordering',
    question: 'What happens after my stock request is approved?',
    answer:
      'Your stock is sent to your assigned pickup point. Confirm receipt in the app once you have it so your inventory count stays accurate.',
  },
  {
    category: 'payments',
    question: 'When do I get paid?',
    answer:
      "Payouts are processed every Friday. Your commission is calculated from the bags you sold during that week's Monday to Thursday sales period.",
  },
  {
    category: 'payments',
    question: 'Where do I see my earnings?',
    answer:
      'Open Wallet from the sidebar for your current balance and your payout history. If a payout looks wrong, raise it with your manager with the dates and figures.',
  },
  {
    category: 'payments',
    question: 'How do I remit cash to the company?',
    answer:
      'Your manager provides the remittance details. After remitting, record it in your daily report and keep your payment receipt until the amount is reflected.',
  },
  {
    category: 'delivery',
    question: 'How do I submit my daily report?',
    answer:
      'Open Daily Report in the sidebar and enter the bags sold, cash collected, and any returns or notes. Reports are due before 8pm each day.',
  },
  {
    category: 'problems',
    question: 'What happens if I miss a daily report?',
    answer:
      'A missed report affects your leaderboard ranking and can delay your payout. If you missed one because of an emergency, tell your manager as soon as you can so it is noted.',
  },
  {
    category: 'problems',
    question: 'How is my leaderboard rank calculated?',
    answer:
      'By total bags sold in the current week. Where two agents are level, the tie is broken by the number of days each reported on time.',
  },
  {
    category: 'problems',
    question: 'My stock count does not match, or items arrived damaged.',
    answer:
      'Contact your manager immediately with your agent code, the date, and a clear description or photo. Do not adjust your report to hide the gap, as it makes the discrepancy harder to trace.',
  },
  {
    category: 'account',
    question: 'How do I log in and update my details?',
    answer:
      'Log in with your registered email address and password. You can update your profile details and change your password from Settings.',
  },
];

const agentGuideSections: HelpGuideSection[] = [
  {
    icon: UserCircle,
    title: 'Getting Started',
    steps: [
      'Log in with your registered email address and password.',
      'Confirm your profile details are correct in Settings.',
      'Use the sidebar to move between your overview, daily reports, orders and wallet.',
    ],
  },
  {
    icon: Package,
    title: 'Requesting and Receiving Stock',
    steps: [
      'Go to Request Stock in the sidebar and enter the quantity you need.',
      'Submit the request so your manager can review and approve it.',
      'Once approved, your stock is delivered to your assigned pickup point.',
      'Confirm receipt in the app so your inventory is updated.',
    ],
  },
  {
    icon: Truck,
    title: 'Submitting Daily Reports',
    steps: [
      'Open Daily Report in the sidebar each day.',
      'Enter the bags sold, total cash collected, and any returns.',
      'Add notes for issues such as damaged stock or delivery problems.',
      'Submit before 8pm. Late or missed reports affect your ranking and payout.',
    ],
  },
  {
    icon: Wallet,
    title: 'Earnings and Commission',
    steps: [
      'Your commission is calculated automatically from your weekly sales.',
      'Check your balance and payout history in the Wallet section.',
      'Payouts run every Friday for the Monday to Thursday sales period.',
      'Raise any payout discrepancy with your manager, with dates and figures.',
    ],
  },
  {
    icon: TrendingUp,
    title: 'Leaderboard and Performance',
    steps: [
      'Your rank is based on total bags sold in the current week.',
      'Ties are broken by the number of days reported on time.',
      'Top-ranked agents may receive bonuses and priority stock allocation.',
      'Consistent reporting plus steady sales is the fastest way to move up.',
    ],
  },
  {
    icon: AlertCircle,
    title: 'Reporting a Problem',
    steps: [
      'For stock discrepancies or damaged items, contact your manager immediately.',
      'Include your agent code, the date, and a clear description of the problem.',
      'If you missed a report because of an emergency, reach out so it can be noted.',
      'Keep receipts for every remittance until the amount is reflected in your wallet.',
    ],
  },
];

export const agentHelpContent: HelpContent = {
  guideTitle: 'Agent Guide',
  guideSubtitle: 'Stock, reporting and payouts explained',
  categories: [
    { key: 'all', label: 'All' },
    { key: 'ordering', label: 'Stock' },
    { key: 'payments', label: 'Earnings' },
    { key: 'delivery', label: 'Reporting' },
    { key: 'problems', label: 'Performance & Issues' },
    { key: 'account', label: 'Account' },
  ],
  faqs: agentFaqs,
  guideSections: agentGuideSections,
};
