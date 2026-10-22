import { useState, useCallback } from 'react';
import { useForm, type UseFormReturn } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import {
  updatePasswordSchema,
  type UpdatePasswordValues,
} from '../../schemas/auth/update-password';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from '../../lib/server-errors';

/*
 * For a logged-in user who knows their current password.
 * Contrast with use-reset-password.ts, which is for a user who does not.
 *
 * One hook, reused by every role's settings screen (and by an app-local "you're still on a temporary password" nag, where one exists) - the fields and rules are identical regardless of who is asking, so there is no role-specific variant of this hook.
 */

export interface UseUpdatePasswordOptions {
  onSuccess?: () => void;
}

export interface UseUpdatePasswordResult {
  form: UseFormReturn<UpdatePasswordValues>;
  submit: (event?: React.BaseSyntheticEvent) => Promise<void>;
  apiError: string | null;
  clearApiError: () => void;
  isSubmitting: boolean;
  done: boolean;
}

export function useUpdatePassword(
  options: UseUpdatePasswordOptions = {},
): UseUpdatePasswordResult {
  const { onSuccess } = options;
  const adapter = useAuthAdapter();

  const [apiError, setApiError] = useState<string | null>(null);
  const [done, setDone] = useState<boolean>(false);

  const form = useForm<UpdatePasswordValues>({
    resolver: zodResolver(updatePasswordSchema),
    mode: 'onChange',
    defaultValues: { currentPassword: '', password: '', confirmPassword: '' },
  });

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  const submit = form.handleSubmit(async (values: UpdatePasswordValues) => {
    setApiError(null);
    try {
      await adapter.updatePassword(values.currentPassword, values.password);
      setDone(true);
      onSuccess?.();
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : 'Could not update your password. Please try again.';

      setApiError(message);
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
