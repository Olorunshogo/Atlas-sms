import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import gsap from 'gsap';
import {
  lockScroll,
  syncSmoothScroll,
  unlockScroll,
  type SmoothScroller,
} from './smooth-scroll';

function fakeLenis(): SmoothScroller & {
  stop: ReturnType<typeof vi.fn>;
  start: ReturnType<typeof vi.fn>;
  raf: ReturnType<typeof vi.fn>;
} {
  return {
    raf: vi.fn(),
    on: vi.fn(() => vi.fn()),
    stop: vi.fn(),
    start: vi.fn(),
  };
}

const teardowns: (() => void)[] = [];

// ScrollTrigger reads matchMedia when it registers; jsdom has none
beforeAll(() => {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
    }),
  });
});

afterEach(() => {
  teardowns.splice(0).forEach((teardown) => teardown());
  document.body.style.overflow = '';
});

describe('smooth scroll', () => {
  it('drives Lenis from the GSAP ticker in milliseconds', () => {
    const lenis = fakeLenis();
    const add = vi.spyOn(gsap.ticker, 'add');
    teardowns.push(syncSmoothScroll(lenis));

    const tick = add.mock.calls[0]?.[0] as (time: number) => void;
    tick(1.5);
    expect(lenis.raf).toHaveBeenCalledWith(1500);
    expect(lenis.on).toHaveBeenCalledWith('scroll', expect.any(Function));
    add.mockRestore();
  });

  it('pauses Lenis and the page while any modal is open, counting stacked locks', () => {
    const lenis = fakeLenis();
    teardowns.push(syncSmoothScroll(lenis));

    lockScroll();
    lockScroll();
    expect(lenis.stop).toHaveBeenCalledTimes(1);
    expect(document.body.style.overflow).toBe('hidden');

    unlockScroll();
    expect(lenis.start).not.toHaveBeenCalled();
    unlockScroll();
    expect(lenis.start).toHaveBeenCalledTimes(1);
    expect(document.body.style.overflow).toBe('');

    unlockScroll();
    expect(lenis.start).toHaveBeenCalledTimes(1);
  });

  it('starts paused when a modal is already open as Lenis mounts', () => {
    lockScroll();
    const lenis = fakeLenis();
    teardowns.push(syncSmoothScroll(lenis));
    expect(lenis.stop).toHaveBeenCalledTimes(1);
    unlockScroll();
    expect(lenis.start).toHaveBeenCalledTimes(1);
  });

  it('removes its ticker and scroll listener on teardown', () => {
    const lenis = fakeLenis();
    const add = vi.spyOn(gsap.ticker, 'add');
    const remove = vi.spyOn(gsap.ticker, 'remove');
    syncSmoothScroll(lenis)();
    expect(remove).toHaveBeenCalledWith(add.mock.calls[0]?.[0]);
    add.mockRestore();
    const off = (lenis.on as ReturnType<typeof vi.fn>).mock.results[0]
      ?.value as ReturnType<typeof vi.fn>;
    expect(off).toHaveBeenCalledTimes(1);
    remove.mockRestore();
  });
});
