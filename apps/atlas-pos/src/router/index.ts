import { createRouter, createWebHistory } from '@ionic/vue-router';
import type { RouteRecordRaw } from 'vue-router';
import { posSession, signedIn } from '../auth/session';

const VERIFY_PATH = '/auth/verify-email';

const routes: RouteRecordRaw[] = [
  { path: '/auth/login', component: () => import('../pages/LoginPage.vue') },
  {
    path: VERIFY_PATH,
    component: () => import('../pages/VerifyEmailPage.vue'),
    meta: { requiresAuth: true },
  },
  {
    path: '/',
    component: () => import('../pages/HomePage.vue'),
    meta: { requiresAuth: true },
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

router.beforeEach((to) => {
  if (!signedIn()) {
    return to.meta.requiresAuth ? '/auth/login' : true;
  }
  if (!posSession.emailVerified) {
    return to.path === VERIFY_PATH ? true : VERIFY_PATH;
  }
  if (to.path === '/auth/login' || to.path === VERIFY_PATH) {
    return '/';
  }
  return true;
});

export default router;
