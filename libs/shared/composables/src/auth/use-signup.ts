import { computed, ref, type ComputedRef, type Ref } from 'vue';
import { useForm, type GenericObject, type TypedSchema } from 'vee-validate';
import { normalizeNigerianPhone, toZod4TypedSchema } from '@atlas/validation';
// RoleSignupConfig/SignupFormValues is the reference source's role-config model
// (buyer, agent, applicant...) - verified to exist at this import path, but still
// worth checking whether it's the right shape for Atlas's actual signup flow.
import type { RoleSignupConfig, SignupFormValues } from '@atlas/types';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from './apply-server-field-errors';

/*
 * Signup for any self-registerable role - ported from the reference source's React
 * use-signup.ts. The role's schema, fields, and redirect all come from the
 * passed-in config, so adding a role needs no change here.
 */

function isUnverifiedEmailError(error: unknown): boolean {
  const body = (error as { body?: { code?: unknown } } | null)?.body;
  return body?.code === 'UNVERIFIED_EMAIL';
}

export interface UseSignupOptions {
  config: RoleSignupConfig;
  onRequiresVerification?: (email: string, role: string) => void;
}

export interface UseSignupResult {
  form: ReturnType<typeof useForm<SignupFormValues>>;
  config: RoleSignupConfig;
  submit: () => Promise<void>;
  apiError: Ref<string | null>;
  clearApiError: () => void;
  /** True once registration succeeded and email verification is pending. */
  awaitingVerification: Ref<boolean>;
  unverifiedEmail: Ref<string | null>;
  goToVerifyEmail: () => void;
}

export function useSignup(options: UseSignupOptions): UseSignupResult {
  const { config, onRequiresVerification } = options;
  const adapter = useAuthAdapter();
  const role = config.role;

  const apiError = ref<string | null>(null);
  const awaitingVerification = ref<boolean>(false);
  const unverifiedEmail = ref<string | null>(null);

  const form = useForm<SignupFormValues>({
    validationSchema: toZod4TypedSchema(config.schema) as TypedSchema<
      GenericObject,
      SignupFormValues
    >,
    initialValues: {
      fullName: '',
      email: '',
      phone: '',
      password: '',
      confirmPassword: '',
      referredByAgentCode: '',
      lga: '',
      address: '',
      cv: undefined,
      acceptedTerms: false,
    },
  });

  function clearApiError(): void {
    apiError.value = null;
  }

  const termsConsent: ComputedRef<Record<string, string | boolean>> = computed(
    (): Record<string, string | boolean> => {
      if (!config.terms) {
        return {};
      }
      return {
        accepted_terms: true,
        terms_document: config.terms.slug,
        terms_version: config.terms.version,
      };
    },
  );

  const handleSubmit = form.handleSubmit(async (values: SignupFormValues) => {
    apiError.value = null;
    unverifiedEmail.value = null;
    const { first_name, last_name } = adapter.splitFullName(
      values.fullName.trim(),
    );

    try {
      if (config.register) {
        await config.register(values, { first_name, last_name, role });
      } else {
        await adapter.register({
          first_name,
          last_name,
          email: values.email,
          password: values.password,
          role,
          phone: values.phone
            ? normalizeNigerianPhone(values.phone)
            : undefined,
          referred_by_agent_code: values.referredByAgentCode || undefined,
          ...termsConsent.value,
        });
      }

      awaitingVerification.value = true;
      if (onRequiresVerification) {
        onRequiresVerification(values.email, role);
      } else {
        adapter.navigate('/verify-email', { email: values.email, role });
      }
    } catch (error) {
      apiError.value =
        error instanceof Error
          ? error.message
          : 'Could not create your account. Please try again.';
      applyServerFieldErrors(error, form.setFieldError);
      if (isUnverifiedEmailError(error)) {
        unverifiedEmail.value = values.email;
      }
    }
  });

  async function submit(): Promise<void> {
    await handleSubmit();
  }

  function goToVerifyEmail(): void {
    if (!unverifiedEmail.value) return;
    adapter.navigate('/verify-email', { email: unverifiedEmail.value, role });
  }

  return {
    form,
    config,
    submit,
    apiError,
    clearApiError,
    awaitingVerification,
    unverifiedEmail,
    goToVerifyEmail,
  };
}
