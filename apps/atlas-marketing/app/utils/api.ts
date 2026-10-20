export interface TenantChoice {
  id: string;
  name: string;
}

export interface SessionUser {
  id: string;
  full_name: string;
  email: string;
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

export async function apiRequest<T>(
  path: string,
  init?: { method?: string; body?: unknown },
): Promise<ApiResult<T>> {
  const response = await $fetch.raw<ApiBody<T>>(`/api/${path}`, {
    method: init?.method ?? 'GET',
    body: init?.body as Record<string, unknown> | undefined,
    ignoreResponseError: true,
  });

  return {
    status: response.status,
    body: response._data ?? {
      message: 'Empty response from the server.',
      success: false,
    },
  };
}

export function isSuccess<T>(body: ApiBody<T>): body is ApiSuccess<T> {
  return body.success === true;
}
