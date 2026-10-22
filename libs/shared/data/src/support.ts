/*
 * The reference source's own contact details (phone/email/social). The TYPE and
 * link-builder helpers are generic and worth keeping; the actual contact
 * values below belong to that source, not Atlas - replace SUPPORT before this is
 * wired into any Atlas footer/contact page.
 */

// === Types

export interface SupportChannels {
  /* E.164, no spaces. Safe for tel: and wa.me. */
  phone: string;
  /* Human-readable form of the same number, for display only. */
  phoneDisplay: string;
  supportEmail: string;
  partnerEmail?: string;
  /* Plain-language availability, shown next to the phone and chat channels. */
  hours: string;
  linkedin: string;
  youtube: string;
}

// === Contacts (the reference source's own, replace before use)

export const SUPPORT: SupportChannels = {
  phone: '+2349139992273',
  phoneDisplay: '09139992273',
  supportEmail: 'contact@example.com',
  hours: 'Mon - Sat, 9am - 5pm',
  linkedin: 'https://www.linkedin.com/',
  youtube: 'https://www.youtube.com/',
};

// === Link builders

export const supportTelHref = `tel:${SUPPORT.phone}`;
export const supportMailtoHref = `mailto:${SUPPORT.supportEmail}`;
export const partnerMailtoHref = `mailto:${SUPPORT.partnerEmail}`;

/*
 * Direct one-to-one chat with support, deliberately not a group invite.
 * Pass `message` to pre-fill the composer.
 */
export function supportWhatsAppHref(message?: string): string {
  const base = `https://wa.me/${SUPPORT.phone}`;
  return message ? `${base}?text=${encodeURIComponent(message)}` : base;
}
