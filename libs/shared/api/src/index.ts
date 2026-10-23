export { atlasRequest, createAtlasClient } from './client';
export { createFetchAdapter } from './fetch-adapter';
export type {
  InProcessFetch,
  InProcessRequest,
  InProcessResponse,
} from './fetch-adapter';
export type { AtlasClientOptions, AtlasRequestConfig } from './client';
export {
  EMAIL_NOT_VERIFIED,
  isEmailNotVerified,
  isSuccess,
  normalizeBody,
} from './envelope';
export type {
  ApiBody,
  ApiFailure,
  ApiResult,
  ApiSuccess,
  IssuedSession,
  SessionProfile,
  SessionUser,
  StoreRecord,
  TenantChoice,
  TenantRecord,
} from './types';
