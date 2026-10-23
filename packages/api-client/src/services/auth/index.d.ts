import type {
  AuthUser,
  LoginResponse,
  RegisterResponse,
  UserRole,
} from '../../types/auth';
export interface LoginPayload {
  email: string;
  password: string;
}
/**
 * Roles a user may create for themselves.
 * Admin is deliberately excluded, and the backend enforces the same allow-list - this type is a convenience, not the security boundary.
 */
export type SelfRegisterableRole = Extract<UserRole, 'buyer' | 'agent'>;
export interface RegisterPayload {
  first_name: string;
  last_name: string;
  email: string;
  password: string;
  role: SelfRegisterableRole;
  phone?: string;
  referred_by_agent_code?: string;
  accepted_terms?: boolean;
  terms_document?: string;
  terms_version?: string;
}
export interface ResetPasswordPayload {
  token: string;
  password: string;
}
export interface VerifyEmailPayload {
  email: string;
  otp: string;
}
export declare function login(payload: LoginPayload): Promise<LoginResponse>;
export declare function adminLogin(
  payload: LoginPayload,
): Promise<LoginResponse>;
export declare function register(payload: RegisterPayload): Promise<
  | RegisterResponse
  | {
      user: AuthUser;
    }
>;
export declare function applyAgent(form: FormData): Promise<{
  id: number;
}>;
export declare function forgotPassword(email: string): Promise<unknown>;
export declare function resetPassword(
  payload: ResetPasswordPayload,
): Promise<unknown>;
export declare function verifyEmail(
  payload: VerifyEmailPayload,
): Promise<Partial<LoginResponse>>;
export declare function resendOtp(email: string): Promise<unknown>;
export declare function resendVerification(email: string): Promise<unknown>;
