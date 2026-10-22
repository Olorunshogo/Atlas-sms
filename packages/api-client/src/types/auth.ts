/*
 * investor, farmer and logistics are reserved, not active roles: no schema, signup config or dashboard exists for them yet.
 * Uncomment as each gets real backend support, matching the reference backend's roles.type.ts.
 *
 * applicant and employee power the careers product.
 * Offer accept flips an applicant to employee, customer_care, or tech from the job.
 */
export type UserRole =
  | 'admin'
  | 'agent'
  | 'buyer'
  | 'company'
  | 'applicant'
  | 'employee'
  | 'customer_care'
  | 'tech';
// | "investor"
// | "farmer"
// | "logistics";

export interface AuthUser {
  id: number;
  first_name: string;
  last_name: string;
  email: string;
  phone?: string;
  role: UserRole;
  is_email_verified: boolean;
  created_at?: string;
}

export interface JwtPayload {
  sub: number;
  email: string;
  role: UserRole;
  iat: number;
  exp: number;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
}

export interface LoginResponse {
  user: AuthUser;
  accessToken: string;
  refreshToken: string;
}

export interface RegisterResponse {
  user: AuthUser;
  accessToken: string;
  refreshToken: string;
}
