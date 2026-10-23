export declare function publicRequest<T = unknown>(
  path: string,
  options?: RequestInit,
): Promise<T>;
/** Convenience wrapper for the common JSON POST case. */
export declare function publicPost<T = unknown>(
  path: string,
  payload: unknown,
): Promise<T>;
/** POST a multipart body, for endpoints that accept a file upload. */
export declare function publicPostForm<T = unknown>(
  path: string,
  form: FormData,
): Promise<T>;
