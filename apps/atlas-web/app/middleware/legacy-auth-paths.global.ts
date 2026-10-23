const legacyAuthPaths: Record<string, string> = {
  '/login': '/auth/login',
  '/signup': '/auth/signup',
  '/verify-email': '/auth/verify-email',
  '/forgot-password': '/auth/forgot-password',
  '/reset-password': '/auth/reset-password',
};

export default defineNuxtRouteMiddleware((to) => {
  const mapped = legacyAuthPaths[to.path];
  if (mapped) {
    return navigateTo({ path: mapped, query: to.query });
  }
  if (to.path.startsWith('/invites/')) {
    return navigateTo({ path: '/auth' + to.path, query: to.query });
  }
});
