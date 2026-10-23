import { ref, type Ref } from 'vue';
import { useForm, type GenericObject, type TypedSchema } from 'vee-validate';
import {
  resetPasswordSchema,
  toZod4TypedSchema,
  type ResetPasswordValues,
} from '@atlas/validation';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from './apply-server-field-errors';

/*
 * Step two of password reset - ported from the reference source's React
 * use-reset-password.ts. Applies full password-strength rules, unlike
 * login: the user is choosing a new password here.
 */

export interface UseResetPasswordOptions {
  /** Prefills the token when it arrives via a link rather than being typed. */
  token?: string;
  onSuccess?: () => void;
}

export interface UseResetPasswordResult {
  form: ReturnType<typeof useForm<ResetPasswordValues>>;
  submit: () => Promise<void>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
  done: Ref<boolean>;
}

export function useResetPassword(
  options: UseResetPasswordOptions = {},
): UseResetPasswordResult {
  const { token = '', onSuccess } = options;
  const adapter = useAuthAdapter();

  const apiError = ref<string | null>(null);
  const done = ref<boolean>(false);

  const form = useForm<ResetPasswordValues>({
    validationSchema: toZod4TypedSchema(resetPasswordSchema) as TypedSchema<
      GenericObject,
      ResetPasswordValues
    >,
    initialValues: { token, password: '', confirmPassword: '' },
  });

  function clearApiError(): void {
    apiError.value = null;
  }

  const handleSubmit = form.handleSubmit(
    async (values: ResetPasswordValues) => {
      apiError.value = null;
      try {
        await adapter.resetPassword(values.token, values.password);
        done.value = true;
        if (onSuccess) {
          onSuccess();
          return;
        }
        adapter.navigate('/login');
      } catch (error) {
        apiError.value =
          error instanceof Error
            ? error.message
            : 'Could not reset your password. Please try again.';
        applyServerFieldErrors(error, form.setFieldError);
      }
    },
  );

  async function submit(): Promise<void> {
    await handleSubmit();
  }

  return { form, submit, apiError, clearApiError, done };
}
