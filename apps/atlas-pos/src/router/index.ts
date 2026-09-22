import { createRouter, createWebHistory } from '@ionic/vue-router';
import type { RouteRecordRaw } from 'vue-router';

/**
 * Lazy-loaded so each page becomes its own chunk as more routes are added -
 * only HomePage exists today, but this is the pattern to keep, not add later.
 */
const routes: RouteRecordRaw[] = [
  { path: '/', component: () => import('../pages/HomePage.vue') },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

export default router;
