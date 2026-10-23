import { useState, useCallback } from 'react';
import { useForm, type UseFormReturn } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useAuthAdapter } from './auth-adapter';
import { applyServerFieldErrors } from '../../lib/server-errors';
import { normalizeNigerianPhone } from '../../schemas/generics';
import type {
  RoleSignupConfig,
  SignupFormValues,
} from '../../types/signup-config';

/*
 * Signup for any self-registerable role.
 * The role's schema, fields, and redirect all come from ROLE_SIGNUP_CONFIG, so adding a role (farmer) needs no change here.
 */

/*
 * The backend reissues a fresh OTP the moment this fires.
 * By the time a caller acts on `unverifiedEmail`, a code is already waiting.
 * Read structurally rather than importing ApiError, to keep the transport layer out of this UI package.
 */
function isUnverifiedEmailError(error: unknown): boolean {
  const body = (error as { body?: { code?: unknown } } | null)?.body;
  return body?.code === 'UNVERIFIED_EMAIL';
}

/*
 * `config`: the role's schema, fields, and redirect all come from its config, so this hook never branches on which role it is handling.
 */
export interface UseSignupOptions {
  config: RoleSignupConfig;
  onRequiresVerification?: (email: string, role: string) => void;
}

export interface UseSignupResult {
  form: UseFormReturn<SignupFormValues>;
  config: RoleSignupConfig;
  submit: (event?: React.BaseSyntheticEvent) => Promise<void>;
  apiError: string | null;
  clearApiError: () => void;
  isSubmitting: boolean;
  /** True once registration succeeded and email verification is pending. */
  awaitingVerification: boolean;
  /*
   * Set when signup failed because this email is already registered but not yet verified.
   * Lets the page offer an explicit way to verify-email instead of a dead-end error.
   */
  unverifiedEmail: string | null;
  goToVerifyEmail: () => void;
}

export function useSignup(options: UseSignupOptions): UseSignupResult {
  const { config, onRequiresVerification } = options;
  const adapter = useAuthAdapter();
  const role = config.role;

  const [apiError, setApiError] = useState<string | null>(null);
  const [awaitingVerification, setAwaitingVerification] =
    useState<boolean>(false);
  const [unverifiedEmail, setUnverifiedEmail] = useState<string | null>(null);

  const form = useForm<SignupFormValues>({
    resolver: zodResolver(config.schema),
    mode: 'onChange',
    defaultValues: {
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

  const clearApiError = useCallback((): void => {
    setApiError(null);
  }, []);

  /*
   * The consent to record, taken from the role's own config rather than from the form.
   * What the user ticked is a boolean; which document that tick refers to is a property of the role, and only the role table knows it.
   */
  const termsConsent = config.terms
    ? {
        accepted_terms: true,
        terms_document: config.terms.slug,
        terms_version: config.terms.version,
      }
    : {};

  const submit = form.handleSubmit(async (values: SignupFormValues) => {
    setApiError(null);
    setUnverifiedEmail(null);
    const { first_name, last_name } = adapter.splitFullName(
      values.fullName.trim(),
    );

    try {
      /*
       * The role decides how its account is created, not this hook.
       * A role with its own endpoint supplies `register` in its config; everything else falls through to the shared one.
       * There is deliberately no branch on which role this is, so a new role is a config entry and nothing here.
       */
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
          ...termsConsent,
        });
      }

      /*
       * Only a genuine success reaches here.
       * `config.register`/`adapter.register` throw on any non-2xx response, including the unverified-email conflict.
       * So this never runs unless the account was actually created just now.
       */
      setAwaitingVerification(true);
      if (onRequiresVerification) {
        onRequiresVerification(values.email, role);
      } else {
        adapter.navigate('/verify-email', { email: values.email, role });
      }
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : 'Could not create your account. Please try again.';

      setApiError(message);
      applyServerFieldErrors(error, form);
      if (isUnverifiedEmailError(error)) {
        setUnverifiedEmail(values.email);
      }
    }
  });

  const goToVerifyEmail = useCallback((): void => {
    if (!unverifiedEmail) return;
    adapter.navigate('/verify-email', { email: unverifiedEmail, role });
  }, [unverifiedEmail, adapter, role]);

  return {
    form,
    config,
    submit,
    apiError,
    clearApiError,
    isSubmitting: form.formState.isSubmitting,
    awaitingVerification,
    unverifiedEmail,
    goToVerifyEmail,
  };
}
