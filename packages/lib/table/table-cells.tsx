import type { ReactNode } from 'react';
import { cn } from '../utils';
import { formatFromKobo } from '../../utils/format-currency';

/*
 * Cell primitives.
 *
 * These exist so money is formatted in one place rather than as a bare `/ 100` at each call site, and so the same "title over subtitle" cell does not get rebuilt with slightly different type scales on every page.
 */

// === Text

export interface TablePrimaryCellProps {
  title: ReactNode;
  subtitle?: ReactNode;
  /** An avatar, thumbnail or icon shown before the text. */
  leading?: ReactNode;
}

export function TablePrimaryCell({
  title,
  subtitle,
  leading,
}: TablePrimaryCellProps) {
  return (
    <div className="flex items-center gap-3">
      {leading}
      <div className="flex min-w-0 flex-col">
        <span className="text-heading truncate font-medium">{title}</span>
        {subtitle !== undefined && subtitle !== null && (
          <span className="text-body truncate text-xs">{subtitle}</span>
        )}
      </div>
    </div>
  );
}

export interface TableTextCellProps {
  value: ReactNode;
  /** Renders when the value is null, undefined or an empty string. */
  fallback?: ReactNode;
  className?: string;
}

export function TableTextCell({
  value,
  fallback = '-',
  className,
}: TableTextCellProps) {
  const isEmpty = value === null || value === undefined || value === '';
  return (
    <span className={cn('text-body', isEmpty && 'opacity-50', className)}>
      {isEmpty ? fallback : value}
    </span>
  );
}

// === Money

export interface TableAmountCellProps {
  /** Minor units, as every money column in this database stores them. */
  kobo: number;
  emphasis?: boolean;
  className?: string;
}

export function TableAmountCell({
  kobo,
  emphasis = true,
  className,
}: TableAmountCellProps) {
  return (
    <span
      className={cn(
        'tabular-nums',
        emphasis ? 'text-heading font-semibold' : 'text-body',
        className,
      )}
    >
      {formatFromKobo(kobo)}
    </span>
  );
}

// === Dates

export interface TableDateCellProps {
  value: string | number | Date | null | undefined;
  /** Adds the time under the date. */
  withTime?: boolean;
  locale?: string;
  className?: string;
}

export function TableDateCell({
  value,
  withTime = false,
  locale = 'en-NG',
  className,
}: TableDateCellProps) {
  if (value === null || value === undefined || value === '') {
    return <TableTextCell value={null} className={className} />;
  }

  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return <TableTextCell value={null} className={className} />;
  }

  const day = date.toLocaleDateString(locale, {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });

  return (
    <span className={cn('text-body flex flex-col text-xs', className)}>
      <span>{day}</span>
      {withTime && (
        <span className="opacity-70">
          {date.toLocaleTimeString(locale, {
            hour: '2-digit',
            minute: '2-digit',
          })}
        </span>
      )}
    </span>
  );
}

// === Status

/*
 * Tones map onto the dashboard status tokens already in the theme, so a badge here matches a badge anywhere else in the app.
 */
export type StatusTone =
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'active'
  | 'neutral';

const toneClasses: Record<StatusTone, string> = {
  success: 'bg-status-delivered text-status-delivered-fg',
  warning: 'bg-status-pending text-status-pending-fg',
  danger: 'bg-status-cancelled text-status-cancelled-fg',
  info: 'bg-status-on-the-way text-status-on-the-way-fg',
  active: 'bg-status-active text-status-active-fg',
  neutral: 'bg-light-bg text-body',
};

export interface TableStatusBadgeProps {
  label: ReactNode;
  tone?: StatusTone;
  icon?: ReactNode;
  className?: string;
}

export function TableStatusBadge({
  label,
  tone = 'neutral',
  icon,
  className,
}: TableStatusBadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold whitespace-nowrap',
        toneClasses[tone],
        className,
      )}
    >
      {icon}
      {label}
    </span>
  );
}
