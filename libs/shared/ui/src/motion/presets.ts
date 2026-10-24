import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

/*
 * The only motion API in Atlas.
 * Components call these presets.
 * A one-off tween in a page is how motion stops feeling like one product.
 */

const DESKTOP_QUERY = '(min-width: 64rem)';

/** Every duration, offset and ease Atlas animates with. Change motion here, never in a component. */
export const motionTokens = {
  easeOut: 'power2.out',
  easeIn: 'power2.in',
  easeOutStrong: 'power3.out',
  revealDuration: 0.6,
  revealY: 40,
  revealStart: 'top 85%',
  dialogBackdropEnter: 0.2,
  dialogPanelEnter: 0.35,
  dialogPanelExit: 0.22,
  dialogBackdropExit: 0.18,
  modalEnterDuration: 0.35,
  modalEnterStagger: 0.06,
  modalEnterY: 12,
  modalExitDuration: 0.18,
  modalExitY: 8,
  // Shorter than Lenis's 1.2s default, same as Celeris: snappy, not syrupy
  smoothScrollDuration: 1,
} as const;

/** Elements a modal staggers in when it opens. */
export const MODAL_FIELD_SELECTOR = '[data-gsap-field]';

let scrollTriggerReady = false;

export function registerScrollTrigger(): void {
  if (scrollTriggerReady || typeof window === 'undefined') {
    return;
  }
  gsap.registerPlugin(ScrollTrigger);
  scrollTriggerReady = true;
}

export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') {
    return true;
  }
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

function panelOffset(): number {
  if (typeof window === 'undefined') {
    return 24;
  }
  if (window.matchMedia(DESKTOP_QUERY).matches) {
    return 24;
  }
  return 48;
}

export function playDialogEnter(
  backdrop: HTMLElement,
  panel: HTMLElement,
): gsap.Context {
  return gsap.context(() => {
    if (prefersReducedMotion()) {
      gsap.set([backdrop, panel], { opacity: 1, y: 0 });
      return;
    }
    const offset = panelOffset();
    gsap.set(backdrop, { opacity: 0 });
    gsap.set(panel, { opacity: 0, y: offset });
    gsap
      .timeline()
      .to(backdrop, {
        opacity: 1,
        duration: motionTokens.dialogBackdropEnter,
        ease: motionTokens.easeOut,
      })
      .to(
        panel,
        {
          opacity: 1,
          y: 0,
          duration: motionTokens.dialogPanelEnter,
          ease: motionTokens.easeOutStrong,
        },
        '<',
      );
  });
}

/*
 * Unmount only from onComplete.
 * Removing the node first skips the exit frames.
 */
export function playDialogExit(
  backdrop: HTMLElement,
  panel: HTMLElement,
  onComplete: () => void,
): gsap.core.Timeline | null {
  if (prefersReducedMotion()) {
    onComplete();
    return null;
  }
  const offset = panelOffset();
  return gsap
    .timeline({ onComplete })
    .to(panel, {
      opacity: 0,
      y: offset,
      duration: motionTokens.dialogPanelExit,
      ease: motionTokens.easeIn,
    })
    .to(
      backdrop,
      { opacity: 0, duration: motionTokens.dialogBackdropExit },
      '<',
    );
}

/** Staggers a modal's `[data-gsap-field]` elements in, after the panel itself has started entering. */
export function playModalFieldsEnter(root: HTMLElement): void {
  const fields = root.querySelectorAll<HTMLElement>(MODAL_FIELD_SELECTOR);
  if (prefersReducedMotion() || fields.length === 0) {
    return;
  }
  gsap.fromTo(
    fields,
    { opacity: 0, y: motionTokens.modalEnterY },
    {
      opacity: 1,
      y: 0,
      duration: motionTokens.modalEnterDuration,
      stagger: motionTokens.modalEnterStagger,
      ease: motionTokens.easeOut,
    },
  );
}

/** Fades a modal's content out, then runs `onComplete`, so the panel's own exit plays after the content is gone. */
export function playModalContentExit(
  root: HTMLElement,
  onComplete: () => void,
): void {
  if (prefersReducedMotion()) {
    onComplete();
    return;
  }
  gsap.to(root, {
    opacity: 0,
    y: motionTokens.modalExitY,
    duration: motionTokens.modalExitDuration,
    ease: motionTokens.easeIn,
    onComplete,
  });
}

/*
 * Each [data-gsap="reveal"] owns its own trigger.
 * One trigger on the page root would fire every section at once.
 */
export function revealOnScroll(root: HTMLElement): () => void {
  registerScrollTrigger();
  const context = gsap.context(() => {
    gsap.matchMedia().add('(prefers-reduced-motion: no-preference)', () => {
      const targets = gsap.utils.toArray<HTMLElement>('[data-gsap="reveal"]');
      targets.forEach((element) => {
        gsap.from(element, {
          y: motionTokens.revealY,
          opacity: 0,
          duration: motionTokens.revealDuration,
          ease: motionTokens.easeOutStrong,
          scrollTrigger: {
            trigger: element,
            start: motionTokens.revealStart,
            toggleActions: 'play none none none',
          },
        });
      });
    });
  }, root);

  return () => {
    context.revert();
  };
}
