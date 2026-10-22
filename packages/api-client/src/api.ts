/*
 * Neither URL is a secret, so both are keyed by Vite's own built-in `mode`
 * rather than a VITE_API_URL that every app would otherwise need its own
 * .env.staging/.env.production to supply. `mode` comes from the --mode flag
 * each app's build command passes (see vercel.json), not from an env file.
 */
const API_URL_BY_MODE: Record<string, string> = {
  production: 'https://api.example.com/api/v1',
  staging: 'https://api-test.example.com/api/v1',
};

export const BASE_BACKEND_URL: string =
  API_URL_BY_MODE[import.meta.env.MODE] ?? 'http://localhost:4002/api/v1';

/* Printed once at module load, on both the server render and the client, so the mode and the URL it resolved to are always visible instead of discovered by accident. */
console.log(
  `[api] mode=${import.meta.env.MODE} apiBaseUrl=${BASE_BACKEND_URL}`,
);

/** Standard envelope returned by every backend endpoint */
export interface ApiResponse<T = unknown> {
  statusCode: number;
  message: string;
  data: T;
  timestamp: string;
  version: string;
  path: string;
}

/** Validation error shape from the backend */
export interface ApiFieldError {
  field: string;
  message: string;
}

/** Error shape returned by failed API responses */
export interface ApiResponseError {
  statusCode: number;
  message: string;
  errors?: ApiFieldError[];
}
