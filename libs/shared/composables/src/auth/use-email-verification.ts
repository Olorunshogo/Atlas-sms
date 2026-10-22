import {
  computed,
  onBeforeUnmount,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from 'vue';
import { useAuthAdapter } from './auth-adapter';

/*
 * OTP email verification - ported from the reference source's React use-email-verification.ts
 * (useState/useEffect -> ref/watch). Not form-schema-based: the UI is a
 * segmented code input driven by one string, not a multi-field form, so a
 * plain ref is the honest fit. Validation is the length/digits check on
 * submit.
 */

export interface UseEmailVerificationOptions {
  email: string;
  onVerified?: (role: string) => void;
  resendCooldownSeconds?: number;
  maxResends?: number;
}

export interface UseEmailVerificationResult {
  code: Ref<string>;
  setCode: (value: string) => void;
  submit: () => Promise<void>;
  resend: () => Promise<void>;
  apiError: Ref<string | null>;
  isSubmitting: Ref<boolean>;
  isResending: Ref<boolean>;
  resent: Ref<boolean>;
  verified: Ref<boolean>;
  cooldown: Ref<number>;
  canResend: ComputedRef<boolean>;
  maxResendsReached: ComputedRef<boolean>;
  isCodeComplete: ComputedRef<boolean>;
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

  const code = ref<string>('');
  const apiError = ref<string | null>(null);
  const isSubmitting = ref<boolean>(false);
  const isResending = ref<boolean>(false);
  const resent = ref<boolean>(false);
  const verified = ref<boolean>(false);
  const cooldown = ref<number>(resendCooldownSeconds);
  const resendCount = ref<number>(0);

  let timer: ReturnType<typeof setInterval> | null = null;

  watch(
    cooldown,
    (value: number) => {
      if (timer) {
        clearInterval(timer);
        timer = null;
      }
      if (value <= 0) return;
      timer = setInterval(() => {
        cooldown.value = cooldown.value <= 1 ? 0 : cooldown.value - 1;
      }, 1000);
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    if (timer) clearInterval(timer);
  });

  const maxResendsReached = computed<boolean>(
    () => resendCount.value >= maxResends,
  );
  const canResend = computed<boolean>(
    () => cooldown.value === 0 && !maxResendsReached.value,
  );
  const isCodeComplete = computed<boolean>(
    () => code.value.length === OTP_LENGTH,
  );

  function setCode(value: string): void {
    code.value = value.replace(/\D/g, '').slice(0, OTP_LENGTH);
    apiError.value = null;
  }

  async function submit(): Promise<void> {
    if (code.value.length !== OTP_LENGTH) {
      apiError.value = `Enter the ${OTP_LENGTH}-digit code`;
      return;
    }

    apiError.value = null;
    isSubmitting.value = true;

    try {
      const session = await adapter.verifyEmail(email, code.value);
      verified.value = true;

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
      apiError.value =
        error instanceof Error
          ? error.message
          : 'Could not verify that code. Please try again.';
    } finally {
      isSubmitting.value = false;
    }
  }

  async function resend(): Promise<void> {
    if (!canResend.value) return;

    apiError.value = null;
    isResending.value = true;
    resent.value = false;

    try {
      await adapter.resendOtp(email);
      resent.value = true;
      resendCount.value += 1;
      cooldown.value = resendCooldownSeconds;
    } catch (error) {
      apiError.value =
        error instanceof Error
          ? error.message
          : 'Could not resend the code. Please try again.';
    } finally {
      isResending.value = false;
    }
  }

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
    isCodeComplete,
  };
}
