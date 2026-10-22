import { useEffect } from 'react';

export function useInjectComponentStyles(styleId: string, css: string): void {
  useEffect(() => {
    if (typeof document === 'undefined') return;
    if (document.getElementById(styleId)) return;
    const el: HTMLStyleElement = document.createElement('style');
    el.id = styleId;
    el.textContent = css;
    document.head.appendChild(el);
  }, [styleId, css]);
}
