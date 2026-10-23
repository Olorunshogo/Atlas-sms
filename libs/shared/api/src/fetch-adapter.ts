import {
  AxiosHeaders,
  type AxiosAdapter,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from 'axios';

export interface InProcessResponse {
  _data: unknown;
  status: number;
  statusText: string;
}

export interface InProcessRequest {
  method: string;
  headers: Record<string, string>;
  body?: string;
  ignoreResponseError: true;
}

export interface InProcessFetch {
  raw: (url: string, init: InProcessRequest) => Promise<InProcessResponse>;
}

/**
 * Server-side adapter so Axios still runs the interceptors without opening a socket back to the app.
 */
export function createFetchAdapter(
  getFetch: () => InProcessFetch,
): AxiosAdapter {
  return async (config) => {
    const requestFetch = getFetch();
    const method = (config.method ?? 'GET').toUpperCase();
    const response = await requestFetch.raw(resolveRequestUrl(config), {
      method,
      headers: headersToRecord(config.headers),
      body:
        method === 'GET' || method === 'HEAD'
          ? undefined
          : serializeBody(config.data),
      ignoreResponseError: true,
    });

    const settled: AxiosResponse = {
      data: response._data,
      status: response.status,
      statusText: response.statusText,
      headers: {},
      config,
    };
    return settled;
  };
}

function resolveRequestUrl(config: InternalAxiosRequestConfig): string {
  const url = config.url ?? '';
  if (url.startsWith('http://') || url.startsWith('https://')) {
    return url;
  }
  const base = config.baseURL ?? '';
  const trimmedBase = base.endsWith('/') ? base.slice(0, -1) : base;
  const trimmedUrl = url.startsWith('/') ? url.slice(1) : url;
  if (!trimmedBase) {
    return `/${trimmedUrl}`;
  }
  return `${trimmedBase}/${trimmedUrl}`;
}

function headersToRecord(
  headers: InternalAxiosRequestConfig['headers'],
): Record<string, string> {
  const record: Record<string, string> = {};
  const json = AxiosHeaders.from(headers).toJSON();
  for (const [key, value] of Object.entries(json)) {
    if (typeof value === 'string') {
      record[key] = value;
      continue;
    }
    if (typeof value === 'number' || typeof value === 'boolean') {
      record[key] = String(value);
    }
  }
  return record;
}

function serializeBody(data: unknown): string | undefined {
  if (data === undefined || data === null) {
    return undefined;
  }
  if (typeof data === 'string') {
    return data;
  }
  return JSON.stringify(data);
}
