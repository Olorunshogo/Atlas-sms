import { ref, type Ref } from 'vue';
import { useForm, type GenericObject, type TypedSchema } from 'vee-validate';
import {
  updatePasswordSchema,
  toZod4TypedSchema,
  type UpdatePasswordValues,
} from '@atlas/validation';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from './apply-server-field-errors';

/*
 * For a logged-in user who knows their current password - contrast with
 * use-reset-password.ts, which is for a user who does not. Ported from
 * the reference source's React use-update-password.ts. One composable, reused by every
 * role's settings screen.
 */

export interface UseUpdatePasswordOptions {
  onSuccess?: () => void;
}

export interface UseUpdatePasswordResult {
  form: ReturnType<typeof useForm<UpdatePasswordValues>>;
  submit: () => Promise<void>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
  done: Ref<boolean>;
}

export function useUpdatePassword(
  options: UseUpdatePasswordOptions = {},
): UseUpdatePasswordResult {
  const { onSuccess } = options;
  const adapter = useAuthAdapter();

  const apiError = ref<string | null>(null);
  const done = ref<boolean>(false);

  const form = useForm<UpdatePasswordValues>({
    validationSchema: toZod4TypedSchema(updatePasswordSchema) as TypedSchema<
      GenericObject,
      UpdatePasswordValues
    >,
    initialValues: { currentPassword: '', password: '', confirmPassword: '' },
  });

  function clearApiError(): void {
    apiError.value = null;
  }

  const handleSubmit = form.handleSubmit(
    async (values: UpdatePasswordValues) => {
      apiError.value = null;
      try {
        await adapter.updatePassword(values.currentPassword, values.password);
        done.value = true;
        onSuccess?.();
      } catch (error) {
        apiError.value =
          error instanceof Error
            ? error.message
            : 'Could not update your password. Please try again.';
        applyServerFieldErrors(error, form.setFieldError);
      }
    },
  );

  async function submit(): Promise<void> {
    await handleSubmit();
  }

  return { form, submit, apiError, clearApiError, done };
}
