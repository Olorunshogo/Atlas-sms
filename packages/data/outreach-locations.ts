import type { SelectOption } from '../types/location';

/*
 * === Outreach location identifiers
 *
 * NOT a geography source - see `nigerian-states.ts` for that. This is the
 * outreach feature's own slug scheme: the `value` on each option is what
 * gets persisted (submitted by the public form in the marketing app,
 * matched against for filtering in the admin app's outreach table), so it
 * cannot be regenerated from `nigerian-states.ts`'s plain display names
 * without risking a mismatch against records already stored under these
 * exact slugs.
 *
 * Two disambiguations exist on purpose and would NOT fall out of a generic
 * slugify(label):
 *   - "Jema'a" -> "jema-a" (apostrophe, not a plain slugify of the label)
 *   - Kaduna North's "Sabo" -> "sabo-north", to avoid colliding with
 *     Chikun's "Sabo" -> "sabo"
 *   - Zaria's "Sabon Gari" -> "sabon-gari-zaria" and "Tudun Wada" ->
 *     "tudun-wada-zaria", to avoid colliding with the top-level "Sabon Gari"
 *     LGA and Kaduna South's "Tudun Wada" area
 *
 * This single copy replaces what used to be two independent app-local
 * `models.ts` files (the marketing app and the admin app) that had to
 * be hand-kept in sync - the actual duplication risk this file removes.
 *
 * Labels here (Kaduna South getting Kakuri/Barnawa/Narayi, not Chikun) match
 * nigerian-states.ts and the backend zone seeder - keep those three in sync
 * if this ever needs a real-geography correction again.
 */

export const kadunaLgas: SelectOption[] = [
  { value: 'chikun', label: 'Chikun' },
  { value: 'kaduna-north', label: 'Kaduna North' },
  { value: 'kaduna-south', label: 'Kaduna South' },
  { value: 'igabi', label: 'Igabi' },
  { value: 'giwa', label: 'Giwa' },
  { value: 'zaria', label: 'Zaria' },
  { value: 'birnin-gwari', label: 'Birnin Gwari' },
  { value: 'ikara', label: 'Ikara' },
  { value: 'jaba', label: 'Jaba' },
  { value: 'jema-a', label: "Jema'a" },
  { value: 'kachia', label: 'Kachia' },
  { value: 'kagarko', label: 'Kagarko' },
  { value: 'kajuru', label: 'Kajuru' },
  { value: 'kaura', label: 'Kaura' },
  { value: 'kauru', label: 'Kauru' },
  { value: 'kubau', label: 'Kubau' },
  { value: 'kudan', label: 'Kudan' },
  { value: 'lere', label: 'Lere' },
  { value: 'makarfi', label: 'Makarfi' },
  { value: 'sabon-gari', label: 'Sabon Gari' },
  { value: 'sanga', label: 'Sanga' },
  { value: 'soba', label: 'Soba' },
  { value: 'zangon-kataf', label: 'Zangon Kataf' },
];

export const kadunaAreasByLga: Record<string, SelectOption[]> = {
  chikun: [
    { value: 'sabon-tasha', label: 'Sabon Tasha' },
    { value: 'sabo', label: 'Sabo' },
    { value: 'kamazou', label: 'Kamazou' },
    { value: 'television', label: 'Television' },
    { value: 'gonin-gora', label: 'Gonin Gora' },
    { value: 'kigo', label: 'Kigo' },
    { value: 'unguwan-dosa', label: 'Unguwan Dosa' },
  ],
  'kaduna-north': [
    { value: 'rigasa', label: 'Rigasa' },
    { value: 'badiko', label: 'Badiko' },
    { value: 'kabala', label: 'Kabala' },
    { value: 'unguwan-sarki', label: 'Unguwan Sarki' },
    { value: 'unguwan-rimi', label: 'Unguwan Rimi' },
    { value: 'sabo-north', label: 'Sabo' },
    { value: 'kawo', label: 'Kawo' },
  ],
  /*
   * Kakuri, Barnawa and Narayi are Kaduna South LGA, not Chikun.
   * That matches nigerian-states.ts and the Kaduna South zone in the seeder.
   * Station Market is the market beside Kaduna Junction, in Makera.
   */
  'kaduna-south': [
    { value: 'kakuri', label: 'Kakuri' },
    { value: 'barnawa', label: 'Barnawa' },
    { value: 'narayi', label: 'Narayi' },
    { value: 'tudun-wada', label: 'Tudun Wada' },
    { value: 'katuru', label: 'Katuru' },
    { value: 'mando', label: 'Mando' },
    { value: 'makera', label: 'Makera' },
    { value: 'station-market', label: 'Station Market' },
    { value: 'unguwan-shanu', label: 'Unguwan Shanu' },
    { value: 'romi', label: 'Romi' },
  ],
  igabi: [
    { value: 'afaka', label: 'Afaka' },
    { value: 'zaria-road', label: 'Zaria Road' },
    { value: 'igabi-town', label: 'Igabi Town' },
    { value: 'jagindi', label: 'Jagindi' },
  ],
  giwa: [
    { value: 'giwa-town', label: 'Giwa Town' },
    { value: 'garun-gwanki', label: 'Garun Gwanki' },
    { value: 'turunku', label: 'Turunku' },
  ],
  zaria: [
    { value: 'zaria-city', label: 'Zaria City' },
    { value: 'sabon-gari-zaria', label: 'Sabon Gari' },
    { value: 'tudun-wada-zaria', label: 'Tudun Wada' },
    { value: 'kwarbai', label: 'Kwarbai' },
  ],
};

/** Flat list of every area, for components that need one combined picker. */
export const kadunaAreas: SelectOption[] =
  Object.values(kadunaAreasByLga).flat();
