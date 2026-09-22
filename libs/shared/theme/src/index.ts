// === Colors
/**
 * Neutral placeholder values (see docs/frontend/design/THEME.md) - swap the
 * underlying CSS vars in global.css when Klassivox picks its own identity,
 * these exports don't need to change.
 */
export const colors = {
  surfaceCanvas: 'var(--surface-canvas)',
  surfaceRaised: 'var(--surface-raised)',
  surfaceSunken: 'var(--surface-sunken)',
  surfaceInked: 'var(--surface-inked)',

  inkPrimary: 'var(--ink-primary)',
  inkSecondary: 'var(--ink-secondary)',
  inkMuted: 'var(--ink-muted)',
  inkDisabled: 'var(--ink-disabled)',
  inkInverse: 'var(--ink-inverse)',

  borderSoft: 'var(--border-soft)',
  borderFirm: 'var(--border-firm)',
  borderInked: 'var(--border-inked)',

  brand: 'var(--brand)',
  brandHover: 'var(--brand-hover)',
  brandSoft: 'var(--brand-soft)',
  brandInk: 'var(--brand-ink)',

  success: 'var(--success)',
  successSoft: 'var(--success-soft)',
  warning: 'var(--warning)',
  warningSoft: 'var(--warning-soft)',
  danger: 'var(--danger)',
  dangerSoft: 'var(--danger-soft)',
} as const;

// === Spacing
export const spacing = {
  '2xs': 'var(--space-2xs)',
  xs: 'var(--space-xs)',
  sm: 'var(--space-sm)',
  md: 'var(--space-md)',
  base: 'var(--space-base)',
  lg: 'var(--space-lg)',
  xl: 'var(--space-xl)',
  '2xl': 'var(--space-2xl)',
  '3xl': 'var(--space-3xl)',
  '4xl': 'var(--space-4xl)',
} as const;
