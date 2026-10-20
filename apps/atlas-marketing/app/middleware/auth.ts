export default defineNuxtRouteMiddleware(async () => {
  const session = useSession();
  if (!session.loaded.value) {
    await session.loadSession();
  }
  if (!session.profile.value) {
    return navigateTo('/login');
  }
});
