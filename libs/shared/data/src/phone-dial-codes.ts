// === Types

export interface PhoneDialCode {
  /** ISO country name, used as the list key. */
  country: string;
  /** E.164 dial code, including the leading "+". */
  dialCode: string;
  flagUrl: string;
}

// === Data

/*
 * The business is Kaduna-based, so this is West-Africa-first with
 * a short tail of other African countries rather than the full ~195-country
 * ITU list. Nigeria is first and is PhoneInputField's default.
 */
export const PHONE_DIAL_CODES = [
  {
    country: 'Nigeria',
    dialCode: '+234',
    flagUrl: 'https://flagcdn.com/w40/ng.png',
  },
  {
    country: 'Ghana',
    dialCode: '+233',
    flagUrl: 'https://flagcdn.com/w40/gh.png',
  },
  {
    country: 'Benin',
    dialCode: '+229',
    flagUrl: 'https://flagcdn.com/w40/bj.png',
  },
  {
    country: 'Togo',
    dialCode: '+228',
    flagUrl: 'https://flagcdn.com/w40/tg.png',
  },
  {
    country: 'Niger',
    dialCode: '+227',
    flagUrl: 'https://flagcdn.com/w40/ne.png',
  },
  {
    country: 'Cameroon',
    dialCode: '+237',
    flagUrl: 'https://flagcdn.com/w40/cm.png',
  },
  {
    country: 'Ivory Coast',
    dialCode: '+225',
    flagUrl: 'https://flagcdn.com/w40/ci.png',
  },
  {
    country: 'Senegal',
    dialCode: '+221',
    flagUrl: 'https://flagcdn.com/w40/sn.png',
  },
  {
    country: 'Mali',
    dialCode: '+223',
    flagUrl: 'https://flagcdn.com/w40/ml.png',
  },
  {
    country: 'Burkina Faso',
    dialCode: '+226',
    flagUrl: 'https://flagcdn.com/w40/bf.png',
  },
  {
    country: 'Chad',
    dialCode: '+235',
    flagUrl: 'https://flagcdn.com/w40/td.png',
  },
  {
    country: 'Kenya',
    dialCode: '+254',
    flagUrl: 'https://flagcdn.com/w40/ke.png',
  },
  {
    country: 'South Africa',
    dialCode: '+27',
    flagUrl: 'https://flagcdn.com/w40/za.png',
  },
  {
    country: 'Egypt',
    dialCode: '+20',
    flagUrl: 'https://flagcdn.com/w40/eg.png',
  },
  {
    country: 'United Kingdom',
    dialCode: '+44',
    flagUrl: 'https://flagcdn.com/w40/gb.png',
  },
  {
    country: 'United States',
    dialCode: '+1',
    flagUrl: 'https://flagcdn.com/w40/us.png',
  },
] as const satisfies readonly PhoneDialCode[];

export const DEFAULT_PHONE_DIAL_CODE: PhoneDialCode = PHONE_DIAL_CODES[0];

/*
 * Longest dial code first, so "+234" is matched before a hypothetical
 * single-digit prefix that happens to also match the start of the string.
 */
const BY_LENGTH_DESC: readonly PhoneDialCode[] = [...PHONE_DIAL_CODES].sort(
  (a, b) => b.dialCode.length - a.dialCode.length,
);

export function findDialCodeForNumber(value: string): PhoneDialCode | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  return (
    BY_LENGTH_DESC.find((entry) => trimmed.startsWith(entry.dialCode)) ?? null
  );
}
