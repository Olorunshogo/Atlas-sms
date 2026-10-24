import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import { registerScrollTrigger } from './presets';

/** The parts of a Lenis instance Atlas drives. Structural, so `@atlas/ui` doesn't depend on lenis. */
export interface SmoothScroller {
  raf: (time: number) => void;
  on: (event: 'scroll', handler: () => void) => unknown;
  stop: () => void;
  start: () => void;
}

const DEFAULT_LAG_THRESHOLD = 500;
const DEFAULT_LAG_ADJUSTED = 33;

let active: SmoothScroller | null = null;
let locks = 0;
let previousOverflow = '';

/**
 * Drives Lenis from the GSAP ticker (Lenis runs with `autoRaf: false`) and keeps ScrollTrigger in step with it,
 * the same wiring as Celeris. Returns the teardown.
 */
export function syncSmoothScroll(scroller: SmoothScroller): () => void {
  registerScrollTrigger();
  active = scroller;
  const off = scroller.on('scroll', ScrollTrigger.update);
  const tick = (time: number): void => {
    scroller.raf(time * 1000);
  };
  gsap.ticker.add(tick);
  gsap.ticker.lagSmoothing(0);
  if (locks > 0) {
    scroller.stop();
  }

  return () => {
    gsap.ticker.remove(tick);
    gsap.ticker.lagSmoothing(DEFAULT_LAG_THRESHOLD, DEFAULT_LAG_ADJUSTED);
    if (typeof off === 'function') {
      off();
    }
    if (active === scroller) {
      active = null;
    }
  };
}

/** Freezes the page under a modal: native overflow and Lenis both. Counted, so stacked modals unlock once. */
export function lockScroll(): void {
  if (locks === 0) {
    previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    active?.stop();
  }
  locks += 1;
}

export function unlockScroll(): void {
  if (locks === 0) {
    return;
  }
  locks -= 1;
  if (locks === 0) {
    document.body.style.overflow = previousOverflow;
    active?.start();
  }
}
