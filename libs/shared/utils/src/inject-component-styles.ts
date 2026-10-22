import { onMounted } from 'vue';

/*
 * Ported from React's useInjectComponentStyles (useEffect-based). Vue has no
 * unmount-time need here - the original never cleaned up on unmount either,
 * since injected styles are meant to persist for the app's lifetime once a
 * component has rendered once - so onMounted alone is the faithful port.
 */
export function useInjectComponentStyles(styleId: string, css: string): void {
  onMounted(() => {
    if (typeof document === 'undefined') return;
    if (document.getElementById(styleId)) return;
    const el: HTMLStyleElement = document.createElement('style');
    el.id = styleId;
    el.textContent = css;
    document.head.appendChild(el);
  });
}
