export type UserRole =
  | 'admin'
  | 'agent'
  | 'buyer'
  | 'company'
  | 'applicant'
  | 'employee'
  | 'customer_care'
  | 'tech';
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
