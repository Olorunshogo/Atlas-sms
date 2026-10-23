import type { SelectOption } from '@atlas/types';

/*
 * From the reference source - field-outreach catchment areas (LGAs/wards in Kaduna)
 * for that source's own agent-outreach feature. Heavy prune candidate: Atlas has
 * no outreach/field-agent feature, and even if a similar area-picker is
 * ever needed, these exact slugs/labels belong to that source, not
 * Atlas's. Ported verbatim for reference only.
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
