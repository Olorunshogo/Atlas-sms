import { createApp } from 'vue';
import { IonicVue } from '@ionic/vue';
import App from './App.vue';
import router from './router';

/* Ionic core CSS - required before any Ion* component renders */
import '@ionic/vue/css/core.css';
import '@ionic/vue/css/normalize.css';
import '@ionic/vue/css/structure.css';
import '@ionic/vue/css/typography.css';

/**
 * Shared theme tokens - same source as atlas-web, so colors/spacing
 * stay identical across web and POS even though the component primitives
 * (Ion* vs reka-ui) differ.
 */
import './assets/css/main.css';

const app = createApp(App).use(IonicVue).use(router);

router.isReady().then(() => {
  app.mount('#app');
});
