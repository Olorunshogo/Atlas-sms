import { nextTick, ref } from 'vue';

/**
 * Pure Vue reactivity + DOM - no Nuxt dependency, unlike useSeoMetaConfig.
 * Framework-agnostic enough to reuse in atlas-mobile later if a
 * Vue-only (non-Ionic) sidebar is ever needed there.
 */

export function useSidebar(closeButtonId?: string) {
  const isSidebarOpen = ref<boolean>(false);

  function openSidebar(): void {
    isSidebarOpen.value = true;
    if (closeButtonId) {
      nextTick(() => {
        document.getElementById(closeButtonId)?.focus();
      });
    }
  }

  function closeSidebar(): void {
    isSidebarOpen.value = false;
  }

  // === Focus trap
  function handleSidebarKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      closeSidebar();
      return;
    }
    if (e.key !== 'Tab') return;

    const container = e.currentTarget as HTMLElement;
    const focusable = Array.from(
      container.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((el) => el.offsetParent !== null);

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (!first || !last) return;

    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  return { isSidebarOpen, openSidebar, closeSidebar, handleSidebarKeydown };
}
