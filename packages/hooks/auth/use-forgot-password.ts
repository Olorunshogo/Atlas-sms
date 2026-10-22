import { useState, useCallback } from 'react';
import { useForm, type UseFormReturn } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import {
  forgotPasswordSchema,
  type ForgotPasswordValues,
} from '../../schemas/auth/password-reset';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from '../../lib/server-errors';

/*
 * Step one of password reset: request the emailed reset token.
 *
 * `sent` is the success state the UI switches on.
 * Deliberately does not reveal whether the email existed - the backend's response is the same either way, and surfacing a difference here would leak account existence.
 */

export interface UseForgotPasswordResult {
  form: UseFormReturn<ForgotPasswordValues>;
  submit: (event?: React.BaseSyntheticEvent) => Promise<void>;
  apiError: string | null;
  clearApiError: () => void;
  isSubmitting: boolean;
  sent: boolean;
  /** Email the token was sent to, for the confirmation copy. */
  sentTo: string | null;
  reset: () => void;
}

export function useForgotPassword(): UseForgotPasswordResult {
  const adapter = useAuthAdapter();
  const [apiError, setApiError] = useState<string | null>(null);
  const [sent, setSent] = useState<boolean>(false);
  const [sentTo, setSentTo] = useState<string | null>(null);

  const form = useForm<ForgotPasswordValues>({
    resolver: zodResolver(forgotPasswordSchema),
    mode: 'onChange',
    defaultValues: { email: '' },
  });

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  const reset = useCallback((): void => {
    setSent(false);
    setSentTo(null);
    setApiError(null);
    form.reset();
  }, [form]);

  const submit = form.handleSubmit(async (values: ForgotPasswordValues) => {
    setApiError(null);
    try {
      await adapter.forgotPassword(values.email);
      setSentTo(values.email);
      setSent(true);
    } catch (error) {
      setApiError(
        error instanceof Error
          ? error.message
          : 'Could not send the reset link. Please try again.',
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
    sent,
    sentTo,
    reset,
  };
}
