import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

/*
 * The only motion API in Atlas.
 * Components call these presets.
 * A one-off tween in a page is how motion stops feeling like one product.
 */

const DESKTOP_QUERY = '(min-width: 64rem)';

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
      .to(backdrop, { opacity: 1, duration: 0.2, ease: 'power2.out' })
      .to(panel, { opacity: 1, y: 0, duration: 0.35, ease: 'power3.out' }, '<');
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
      duration: 0.22,
      ease: 'power2.in',
    })
    .to(backdrop, { opacity: 0, duration: 0.18 }, '<');
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
          y: 40,
          opacity: 0,
          duration: 0.6,
          ease: 'power3.out',
          scrollTrigger: {
            trigger: element,
            start: 'top 85%',
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
