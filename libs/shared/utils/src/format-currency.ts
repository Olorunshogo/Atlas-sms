/*
 * Currency formatting for the whole workspace.
 *
 * Built on Intl.NumberFormat, which is part of the JavaScript standard
 * (ECMA-402) and already implements the international rules we want: ISO 4217
 * currency codes, CLDR locale data, correct symbol placement, and per-locale
 * digit grouping. No third-party dependency is needed or wanted for formatting.
 *
 * `currency` and `locale` are defaulted parameters rather than hardcoded, so
 * moving beyond NGN is a call-site option, not a rewrite.
 */

export interface CurrencyFormatOptions {
  /** ISO 4217 code, e.g. 'NGN', 'USD', 'GBP'. */
  currency?: string;
  /** BCP 47 tag, e.g. 'en-NG', 'en-US'. */
  locale?: string;
  /**
   * Digits after the decimal separator. Defaults to whole units, since most
   * prices in this app are whole naira. Pass 2 where kobo precision matters,
   * such as till reconciliation and refund ledgers.
   */
  fractionDigits?: number;
}

export const DEFAULT_CURRENCY = 'NGN';
export const DEFAULT_LOCALE = 'en-NG';

// === Core

export function formatCurrency(
  amount: number,
  options: CurrencyFormatOptions = {},
): string {
  const {
    currency = DEFAULT_CURRENCY,
    locale = DEFAULT_LOCALE,
    fractionDigits = 0,
  } = options;

  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  }).format(amount);
}

// === Minor units

/*
 * How many minor units make one major unit, as an exponent. NGN and USD are 2
 * (100 kobo / cents), JPY is 0, KWD is 3. Asking Intl rather than assuming 2
 * is what makes this correct for any currency.
 *
 * Cached because resolvedOptions() is not free and this is called per render.
 */
const minorUnitCache = new Map<string, number>();

export function minorUnitExponent(currency = DEFAULT_CURRENCY): number {
  const cached = minorUnitCache.get(currency);
  if (cached !== undefined) return cached;

  const resolved = new Intl.NumberFormat(DEFAULT_LOCALE, {
    style: 'currency',
    currency,
  }).resolvedOptions();

  const exponent = resolved.maximumFractionDigits ?? 2;
  minorUnitCache.set(currency, exponent);
  return exponent;
}

/**
 * Formats a value stored in minor units (kobo, cents). Prices are stored as
 * integers in this codebase, so this is the function most call sites want -
 * it makes 'this number is in minor units' explicit instead of leaving a bare
 * `/ 100` at the call site.
 */
export function formatFromMinorUnits(
  value: number,
  options: CurrencyFormatOptions = {},
): string {
  const { currency = DEFAULT_CURRENCY } = options;
  const major = value / 10 ** minorUnitExponent(currency);
  return formatCurrency(major, options);
}

/** Naira-specific alias, for columns named `*_kobo`. */
export function formatFromKobo(
  kobo: number,
  options: CurrencyFormatOptions = {},
): string {
  return formatFromMinorUnits(kobo, options);
}

/** Converts major units to exact integer minor units - see toMinorUnits note. */
export function toMinorUnits(
  amount: number,
  currency = DEFAULT_CURRENCY,
): number {
  /*
   * Math.round matters. A price held as a float (12.34) is not exactly
   * representable in binary, so multiplying by 100 can land on 1233.9999...
   * Rounding recovers the exact integer, which is what makes summing safe.
   */
  return Math.round(amount * 10 ** minorUnitExponent(currency));
}

/**
 * Sums money without float drift by adding in minor units and converting once
 * at the end. Summing major-unit floats accumulates error; this does not.
 */
export function sumMinorUnits(values: readonly number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

// === Variants

/** Signed, for ledgers and transaction lists: +NGN450,000 / -NGN450,000. */
export function formatSignedCurrency(
  amount: number,
  options: CurrencyFormatOptions = {},
): string {
  const sign = amount < 0 ? '-' : '+';
  return `${sign}${formatCurrency(Math.abs(amount), options)}`;
}

/*
 * Compact form for dashboard tiles and charts: NGN480K, NGN1.2M, NGN2.4B.
 * Uses Intl's own compact notation so the suffixes localise correctly rather
 * than being hardcoded English K/M/B.
 */
export function formatCurrencyCompact(
  amount: number,
  options: CurrencyFormatOptions = {},
): string {
  const { currency = DEFAULT_CURRENCY, locale = DEFAULT_LOCALE } = options;

  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(amount);
}
