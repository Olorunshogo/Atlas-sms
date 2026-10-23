import type { ApiBody, ApiFailure, ApiResult } from './types';

export const EMAIL_NOT_VERIFIED = 'EMAIL_NOT_VERIFIED';

const UNEXPECTED_MESSAGE = 'Unexpected response from the server.';

/**
 * Atlas handlers already return `{ success, message, data?, code? }`.
 * Callers still need that shape when the body is empty, HTML, or a transport failure.
 */
export function normalizeBody<T>(payload: unknown): ApiBody<T> {
  if (
    !isRecord(payload) ||
    typeof payload.success !== 'boolean' ||
    typeof payload.message !== 'string'
  ) {
    return { success: false, message: UNEXPECTED_MESSAGE };
  }

  if (payload.success) {
    return {
      success: true,
      message: payload.message,
      data: payload.data as T,
    };
  }

  const failure: ApiFailure = {
    success: false,
    message: payload.message,
  };
  if (typeof payload.code === 'string') {
    failure.code = payload.code;
  }
  if (isRecord(payload.data)) {
    failure.data = payload.data as ApiFailure['data'];
  }
  return failure;
}

export function isSuccess<T>(
  body: ApiBody<T>,
): body is Extract<ApiBody<T>, { success: true }> {
  return body.success === true;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/** The account behind this session hasn't verified its email, so tenant endpoints are closed to it. */
export function isEmailNotVerified(result: ApiResult<unknown>): boolean {
  return (
    result.status === 403 &&
    !result.body.success &&
    result.body.code === EMAIL_NOT_VERIFIED
  );
}
