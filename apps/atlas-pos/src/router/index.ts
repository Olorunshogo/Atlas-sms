import { createRouter, createWebHistory } from '@ionic/vue-router';
import type { RouteRecordRaw } from 'vue-router';
import { signedIn } from '../auth/session';

const routes: RouteRecordRaw[] = [
  { path: '/auth/login', component: () => import('../pages/LoginPage.vue') },
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
  if (to.meta.requiresAuth && !signedIn()) {
    return '/auth/login';
  }
  if (to.path === '/auth/login' && signedIn()) {
    return '/';
  }
  return true;
});

export default router;
