import { useState, useCallback } from 'react';
import { useForm, type UseFormReturn } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import {
  resetPasswordSchema,
  type ResetPasswordValues,
} from '../../schemas/auth/password-reset';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from '../../lib/server-errors';

/*
 * Step two of password reset.
 * Applies the full password strength rules, unlike login: the user is choosing a new password here, so it must satisfy what the backend will accept.
 */

export interface UseResetPasswordOptions {
  /** Prefills the token when it arrives via a link rather than being typed. */
  token?: string;
  onSuccess?: () => void;
}

export interface UseResetPasswordResult {
  form: UseFormReturn<ResetPasswordValues>;
  submit: (event?: React.BaseSyntheticEvent) => Promise<void>;
  apiError: string | null;
  clearApiError: () => void;
  isSubmitting: boolean;
  done: boolean;
}

export function useResetPassword(
  options: UseResetPasswordOptions = {},
): UseResetPasswordResult {
  const { token = '', onSuccess } = options;
  const adapter = useAuthAdapter();

  const [apiError, setApiError] = useState<string | null>(null);
  const [done, setDone] = useState<boolean>(false);

  const form = useForm<ResetPasswordValues>({
    resolver: zodResolver(resetPasswordSchema),
    mode: 'onChange',
    defaultValues: { token, password: '', confirmPassword: '' },
  });

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  const submit = form.handleSubmit(async (values: ResetPasswordValues) => {
    setApiError(null);
    try {
      await adapter.resetPassword(values.token, values.password);
      setDone(true);
      if (onSuccess) {
        onSuccess();
        return;
      }
      adapter.navigate('/login');
    } catch (error) {
      setApiError(
        error instanceof Error
          ? error.message
          : 'Could not reset your password. Please try again.',
      );
      applyServerFieldErrors(error, form);
    }
  });

  return {
    form,
    submit,
    apiError,
    clearApiError,
    isSubmitting: form.formState.isSubmitting,
    done,
  };
}
