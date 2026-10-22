import { useCallback, useState } from 'react';
import { extractServerFieldErrors } from '../server-errors';

/*
 * Tracks one async dialog action through its full lifecycle.
 *
 * Every dialog that submits something uses this instead of its own loading and error useState pair, so the four states are handled identically everywhere.
 * The 'success' state exists to be rendered (see DialogSuccessPanel) rather than skipped by closing the dialog the moment the promise resolves.
 */

export type DialogSubmissionStatus =
  | 'idle'
  | 'submitting'
  | 'error'
  | 'success';

/*
 * `data` is the value the action resolved with, for success copy that needs it.
 * `fieldErrors` is whatever the backend attributed to specific fields on the last failure (camelCased, keyed the same way `extractServerFieldErrors` keys them), for a dialog with more than one input to show inline instead of only in the top banner.
 */
export interface UseDialogSubmissionResult<T> {
  status: DialogSubmissionStatus;
  error: string | null;
  fieldErrors: Record<string, string>;
  data: T | null;
  run: (action: () => Promise<T>) => Promise<T | undefined>;
  reset: () => void;
  isSubmitting: boolean;
}

export function useDialogSubmission<T = void>(): UseDialogSubmissionResult<T> {
  const [status, setStatus] = useState<DialogSubmissionStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [data, setData] = useState<T | null>(null);

  const reset = useCallback((): void => {
    setStatus('idle');
    setError(null);
    setFieldErrors({});
    setData(null);
  }, []);

  const run = useCallback(
    async (action: () => Promise<T>): Promise<T | undefined> => {
      setStatus('submitting');
      setError(null);
      setFieldErrors({});

      try {
        const result = await action();
        setData(result);
        setStatus('success');
        return result;
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : 'Something went wrong. Please try again.',
        );
        setFieldErrors(extractServerFieldErrors(caught));
        setStatus('error');
        return undefined;
      }
    },
    [],
  );

  return {
    status,
    error,
    fieldErrors,
    data,
    run,
    reset,
    isSubmitting: status === 'submitting',
  };
}
