export interface TenantChoice {
  id: string;
  name: string;
}

export interface SessionUser {
  id: string;
  full_name: string;
  email: string;
  /** `false` until the user verifies; tenant endpoints answer `403 EMAIL_NOT_VERIFIED` meanwhile. */
  email_verified: boolean;
}

/** `data` of login, refresh and switch-tenant. Tokens are only in the body for `x-client-type: app`. */
export interface IssuedSession {
  access_token?: string;
  refresh_token?: string;
  tenant_id: string;
  user: SessionUser;
}

export interface SessionProfile {
  user: SessionUser;
  tenant_id: string;
  is_platform_admin: boolean;
  tenants: TenantChoice[];
}

export interface StoreRecord {
  id: string;
  tenant_id: string;
  name: string;
  slug: string;
  currency: string;
  is_active: boolean;
}

export interface TenantRecord {
  id: string;
  name: string;
  status: string;
}

export interface ApiSuccess<T> {
  message: string;
  data: T;
  success: true;
}

export interface ApiFailure {
  message: string;
  success: false;
  code?: string;
  data?: {
    tenants?: TenantChoice[];
  };
}

export type ApiBody<T> = ApiSuccess<T> | ApiFailure;

export interface ApiResult<T> {
  status: number;
  body: ApiBody<T>;
}
