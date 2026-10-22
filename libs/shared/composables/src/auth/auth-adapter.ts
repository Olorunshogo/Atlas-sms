import { inject, provide, type InjectionKey } from 'vue';

/*
 * Dependency injection boundary for the shared auth composables, ported from
 * the reference source's React auth-adapter.tsx.
 *
 * The composables live here so any Atlas app can import them, but this
 * package must never import @atlas/api, vue-router, or an app's own session
 * state directly - doing so would drag transport and routing into a shared
 * UI-adjacent package. Each app supplies an AuthAdapter once via
 * provideAuthAdapter(), and the composables stay pure: form state,
 * validation, and error mapping only.
 */

export type LoginVariant = 'public' | 'admin';

export interface RegisterPayload {
  first_name: string;
  last_name: string;
  email: string;
  password: string;
  role: string;
  phone?: string;
  referred_by_agent_code?: string;
  accepted_terms?: boolean;
  terms_document?: string;
  terms_version?: string;
}

export interface VerifiedSession {
  accessToken?: string;
  refreshToken?: string;
}

export interface AuthAdapter {
  login: (
    email: string,
    password: string,
    variant: LoginVariant,
  ) => Promise<string>;
  register: (payload: RegisterPayload) => Promise<unknown>;
  forgotPassword: (email: string) => Promise<unknown>;
  resetPassword: (token: string, password: string) => Promise<unknown>;
  updatePassword: (
    currentPassword: string,
    newPassword: string,
  ) => Promise<unknown>;
  verifyEmail: (email: string, otp: string) => Promise<VerifiedSession>;
  resendOtp: (email: string) => Promise<unknown>;
  storeSession: (session: VerifiedSession) => string | null;
  navigate: (path: string, state?: Record<string, unknown>) => void;
  redirectAfterAuth: (role: string) => void;
  splitFullName: (fullName: string) => {
    first_name: string;
    last_name: string;
  };
}

const AUTH_ADAPTER_KEY: InjectionKey<AuthAdapter> = Symbol('AtlasAuthAdapter');

export function provideAuthAdapter(adapter: AuthAdapter): void {
  provide(AUTH_ADAPTER_KEY, adapter);
}

export function useAuthAdapter(): AuthAdapter {
  const adapter = inject(AUTH_ADAPTER_KEY);
  if (!adapter) {
    throw new Error(
      'Auth composables require provideAuthAdapter() to run higher up the component tree.',
    );
  }
  return adapter;
}
