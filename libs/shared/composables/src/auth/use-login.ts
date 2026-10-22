import { ref, type Ref } from 'vue';
import { useForm, type GenericObject, type TypedSchema } from 'vee-validate';
import {
  loginSchema,
  toZod4TypedSchema,
  type LoginValues,
} from '@atlas/validation';
import { useAuthAdapter, type LoginVariant } from './auth-adapter';
import { applyServerFieldErrors } from './apply-server-field-errors';

/*
 * Owns login form state, validation, error mapping, and the post-login
 * redirect. The network call and navigation come from the adapter, so this
 * file contains no fetch/axios/router reference - ported from the reference source's
 * React use-login.ts (react-hook-form -> vee-validate).
 */

function isUnverifiedEmailError(error: unknown): boolean {
  const body = (error as { body?: { code?: unknown } } | null)?.body;
  return body?.code === 'UNVERIFIED_EMAIL';
}

function extractPendingAgentAppliedAt(error: unknown): string | null {
  const body = (
    error as {
      body?: { code?: unknown; data?: { applied_at?: unknown } };
    } | null
  )?.body;
  if (body?.code !== 'AGENT_PENDING_APPROVAL') return null;
  return typeof body.data?.applied_at === 'string'
    ? body.data.applied_at
    : null;
}

export interface UseLoginOptions {
  variant?: LoginVariant;
  onSuccess?: (role: string) => void;
  defaultValues?: Partial<LoginValues>;
}

export interface UseLoginResult {
  form: ReturnType<typeof useForm<LoginValues>>;
  submit: () => Promise<void>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
  unverifiedEmail: Ref<string | null>;
  goToVerifyEmail: () => void;
  pendingAgentAppliedAt: Ref<string | null>;
}

export function useLogin(options: UseLoginOptions = {}): UseLoginResult {
  const { variant = 'public', onSuccess, defaultValues } = options;
  const adapter = useAuthAdapter();

  const apiError = ref<string | null>(null);
  const unverifiedEmail = ref<string | null>(null);
  const pendingAgentAppliedAt = ref<string | null>(null);

  const form = useForm<LoginValues>({
    validationSchema: toZod4TypedSchema(loginSchema) as TypedSchema<
      GenericObject,
      LoginValues
    >,
    initialValues: {
      email: defaultValues?.email ?? '',
      password: defaultValues?.password ?? '',
    },
  });

  function clearApiError(): void {
    apiError.value = null;
  }

  const handleSubmit = form.handleSubmit(async (values: LoginValues) => {
    apiError.value = null;
    unverifiedEmail.value = null;
    pendingAgentAppliedAt.value = null;
    try {
      const role = await adapter.login(values.email, values.password, variant);
      if (onSuccess) {
        onSuccess(role);
        return;
      }
      adapter.redirectAfterAuth(role);
    } catch (error) {
      apiError.value =
        error instanceof Error
          ? error.message
          : 'Something went wrong. Please try again.';
      applyServerFieldErrors(error, form.setFieldError);
      if (isUnverifiedEmailError(error)) {
        unverifiedEmail.value = values.email;
      }
      pendingAgentAppliedAt.value = extractPendingAgentAppliedAt(error);
    }
  });

  async function submit(): Promise<void> {
    await handleSubmit();
  }

  function goToVerifyEmail(): void {
    if (!unverifiedEmail.value) return;
    adapter.navigate('/verify-email', { email: unverifiedEmail.value });
  }

  return {
    form,
    submit,
    apiError,
    clearApiError,
    unverifiedEmail,
    goToVerifyEmail,
    pendingAgentAppliedAt,
  };
}
