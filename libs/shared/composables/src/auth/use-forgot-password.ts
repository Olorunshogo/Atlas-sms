import { ref, type Ref } from 'vue';
import { useForm, type GenericObject, type TypedSchema } from 'vee-validate';
import {
  forgotPasswordSchema,
  toZod4TypedSchema,
  type ForgotPasswordValues,
} from '@atlas/validation';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from './apply-server-field-errors';

/*
 * Step one of password reset: request the emailed reset token. `sent` is the
 * success state the UI switches on. Deliberately does not reveal whether the
 * email existed - ported from the reference source's React use-forgot-password.ts.
 */

export interface UseForgotPasswordResult {
  form: ReturnType<typeof useForm<ForgotPasswordValues>>;
  submit: () => Promise<void>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
  sent: Ref<boolean>;
  sentTo: Ref<string | null>;
  reset: () => void;
}

export function useForgotPassword(): UseForgotPasswordResult {
  const adapter = useAuthAdapter();
  const apiError = ref<string | null>(null);
  const sent = ref<boolean>(false);
  const sentTo = ref<string | null>(null);

  const form = useForm<ForgotPasswordValues>({
    validationSchema: toZod4TypedSchema(forgotPasswordSchema) as TypedSchema<
      GenericObject,
      ForgotPasswordValues
    >,
    initialValues: { email: '' },
  });

  function clearApiError(): void {
    apiError.value = null;
  }

  function reset(): void {
    sent.value = false;
    sentTo.value = null;
    apiError.value = null;
    form.resetForm();
  }

  const handleSubmit = form.handleSubmit(
    async (values: ForgotPasswordValues) => {
      apiError.value = null;
      try {
        await adapter.forgotPassword(values.email);
        sentTo.value = values.email;
        sent.value = true;
      } catch (error) {
        apiError.value =
          error instanceof Error
            ? error.message
            : 'Could not send the reset link. Please try again.';
        applyServerFieldErrors(error, form.setFieldError);
      }
    },
  );

  async function submit(): Promise<void> {
    await handleSubmit();
  }

  return { form, submit, apiError, clearApiError, sent, sentTo, reset };
}
