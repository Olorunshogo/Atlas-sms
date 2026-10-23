import { AUTH_CLIENT_KEY, type AuthClient } from '@atlas/ui-auth';
import { apiRequest } from '../utils/api';

// The shared auth forms talk to `/api/v1`-relative paths; here they go through the Nitro cookie proxy
export default defineNuxtPlugin((nuxtApp) => {
  const client: AuthClient = {
    request: (path, init) => apiRequest(`v1/${path}`, init),
  };
  nuxtApp.vueApp.provide(AUTH_CLIENT_KEY, client);
});
