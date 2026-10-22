/*
 * Fixed allowlist for job-posting tech badges.
 * Keys are what the API stores; icon is an Iconify id for the job detail page.
 */

export const techStackKeys = [
  'typescript',
  'react',
  'nestjs',
  'node',
  'postgres',
  'drizzle',
  'tailwind',
  'vite',
  'redis',
  'docker',
] as const;

export type TechStackKey = (typeof techStackKeys)[number];

export interface TechStackItem {
  key: TechStackKey;
  label: string;
  icon: string;
}

export const techStackCatalog: readonly TechStackItem[] = [
  { key: 'typescript', label: 'TypeScript', icon: 'logos:typescript-icon' },
  { key: 'react', label: 'React', icon: 'logos:react' },
  { key: 'nestjs', label: 'NestJS', icon: 'logos:nestjs' },
  { key: 'node', label: 'Node.js', icon: 'logos:nodejs-icon' },
  { key: 'postgres', label: 'Postgres', icon: 'logos:postgresql' },
  { key: 'drizzle', label: 'Drizzle', icon: 'lucide:database' },
  { key: 'tailwind', label: 'Tailwind', icon: 'logos:tailwindcss-icon' },
  { key: 'vite', label: 'Vite', icon: 'logos:vitejs' },
  { key: 'redis', label: 'Redis', icon: 'logos:redis' },
  { key: 'docker', label: 'Docker', icon: 'logos:docker-icon' },
];

const byKey = new Map(
  techStackCatalog.map((item) => [item.key, item] as const),
);

export function resolveTechStack(
  keys: readonly string[] | null | undefined,
): TechStackItem[] {
  if (!keys?.length) return [];
  const seen = new Set<string>();
  const resolved: TechStackItem[] = [];
  for (const key of keys) {
    if (seen.has(key)) continue;
    seen.add(key);
    const item = byKey.get(key as TechStackKey);
    if (item) resolved.push(item);
  }
  return resolved;
}

export const techStackKeySet: ReadonlySet<string> = new Set(techStackKeys);
