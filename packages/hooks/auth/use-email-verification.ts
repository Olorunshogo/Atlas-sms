import { useState, useCallback, useEffect } from 'react';
import { useAuthAdapter } from './auth-adapter';

/*
 * OTP email verification. Not RHF-based: the UI is a segmented code input driven by one string, not a multi-field form, so a plain controlled value is the honest fit.
 * Validation is the length/digits check on submit.
 *
 * Some backends return a session on successful verification and some do not, so tokens are stored only when present and the user is redirected to login otherwise.
 */

/*
 * `resendCooldownSeconds` is the number of seconds before a resend is allowed again.
 * `maxResends` is the cap on resends before the user must contact support.
 */
export interface UseEmailVerificationOptions {
  email: string;
  onVerified?: (role: string) => void;
  resendCooldownSeconds?: number;
  maxResends?: number;
}

/*
 * `cooldown` is seconds remaining before resend is allowed; 0 means allowed now.
 * `isCodeComplete` is true once every digit is entered, for gating the submit button.
 */
export interface UseEmailVerificationResult {
  code: string;
  setCode: (value: string) => void;
  submit: () => Promise<void>;
  resend: () => Promise<void>;
  apiError: string | null;
  isSubmitting: boolean;
  isResending: boolean;
  resent: boolean;
  verified: boolean;
  cooldown: number;
  canResend: boolean;
  maxResendsReached: boolean;
  isCodeComplete: boolean;
}

const OTP_LENGTH = 6;

export function useEmailVerification(
  options: UseEmailVerificationOptions,
): UseEmailVerificationResult {
  const {
    email,
    onVerified,
    resendCooldownSeconds = 60,
    maxResends = 3,
  } = options;
  const adapter = useAuthAdapter();

  const [code, setCodeState] = useState<string>('');
  const [apiError, setApiError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [isResending, setIsResending] = useState<boolean>(false);
  const [resent, setResent] = useState<boolean>(false);
  const [verified, setVerified] = useState<boolean>(false);
  const [cooldown, setCooldown] = useState<number>(resendCooldownSeconds);
  const [resendCount, setResendCount] = useState<number>(0);

  useEffect(() => {
    if (cooldown <= 0) return;
    const timer = window.setInterval(() => {
      setCooldown((previous) => (previous <= 1 ? 0 : previous - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [cooldown]);

  const maxResendsReached = resendCount >= maxResends;
  const canResend = cooldown === 0 && !maxResendsReached;

  const setCode = useCallback((value: string): void => {
    setCodeState(value.replace(/\D/g, '').slice(0, OTP_LENGTH));
    setApiError(null);
  }, []);

  const submit = useCallback(async (): Promise<void> => {
    if (code.length !== OTP_LENGTH) {
      setApiError(`Enter the ${OTP_LENGTH}-digit code`);
      return;
    }

    setApiError(null);
    setIsSubmitting(true);

    try {
      const session = await adapter.verifyEmail(email, code);
      setVerified(true);

      const role = adapter.storeSession(session);
      if (role) {
        if (onVerified) {
          onVerified(role);
          return;
        }
        adapter.redirectAfterAuth(role);
        return;
      }

      adapter.navigate('/login');
    } catch (error) {
      setApiError(
        error instanceof Error
          ? error.message
          : 'Could not verify that code. Please try again.',
      );
    } finally {
      setIsSubmitting(false);
    }
  }, [code, email, adapter, onVerified]);

  const resend = useCallback(async (): Promise<void> => {
    if (!canResend) return;

    setApiError(null);
    setIsResending(true);
    setResent(false);

    try {
      await adapter.resendOtp(email);
      setResent(true);
      setResendCount((previous) => previous + 1);
      setCooldown(resendCooldownSeconds);
    } catch (error) {
      setApiError(
        error instanceof Error
          ? error.message
          : 'Could not resend the code. Please try again.',
      );
    } finally {
      setIsResending(false);
    }
  }, [email, adapter, canResend, resendCooldownSeconds]);

  return {
    code,
    setCode,
    submit,
    resend,
    apiError,
    isSubmitting,
    isResending,
    resent,
    verified,
    cooldown,
    canResend,
    maxResendsReached,
    isCodeComplete: code.length === OTP_LENGTH,
  };
}
