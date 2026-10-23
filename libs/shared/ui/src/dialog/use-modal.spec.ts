import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';

const tweens = vi.hoisted(() => ({
  to: vi.fn(),
  fromTo: vi.fn(),
}));

vi.mock('gsap', () => ({ default: { to: tweens.to, fromTo: tweens.fromTo } }));
vi.mock('gsap/ScrollTrigger', () => ({ ScrollTrigger: {} }));

import { useModal } from './use-modal';

function motionAllowed(): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({ matches: false, media: query }),
  });
}

function bodyWithFields(count: number): HTMLElement {
  const root = document.createElement('div');
  for (let i = 0; i < count; i++) {
    const field = document.createElement('div');
    field.setAttribute('data-gsap-field', '');
    root.appendChild(field);
  }
  return root;
}

beforeEach(() => {
  motionAllowed();
  tweens.to.mockReset();
  tweens.fromTo.mockReset();
});

describe('useModal', () => {
  it('staggers in every data-gsap-field element', () => {
    const body = ref<HTMLElement | null>(bodyWithFields(3));
    const { playEnter } = useModal(ref<boolean>(true), body);
    playEnter();

    expect(tweens.fromTo).toHaveBeenCalledTimes(1);
    const [targets, from, to] = tweens.fromTo.mock.calls[0] ?? [];
    expect((targets as NodeListOf<HTMLElement>).length).toBe(3);
    expect(from).toMatchObject({ opacity: 0, y: 12 });
    expect(to).toMatchObject({ opacity: 1, y: 0, stagger: 0.06 });
  });

  it('keeps the modal open until the content has faded out', () => {
    const open = ref<boolean>(true);
    const { requestClose } = useModal(
      open,
      ref<HTMLElement | null>(bodyWithFields(1)),
    );
    requestClose();

    expect(open.value).toBe(true);
    const vars = tweens.to.mock.calls[0]?.[1] as {
      opacity: number;
      onComplete: () => void;
    };
    expect(vars.opacity).toBe(0);
    vars.onComplete();
    expect(open.value).toBe(false);
  });

  it('ignores close requests while blocked', () => {
    const open = ref<boolean>(true);
    const blocked = ref<boolean>(true);
    const { requestClose } = useModal(
      open,
      ref<HTMLElement | null>(bodyWithFields(1)),
      {
        isBlocked: () => blocked.value,
      },
    );
    requestClose();
    expect(tweens.to).not.toHaveBeenCalled();
    expect(open.value).toBe(true);

    blocked.value = false;
    requestClose();
    expect(tweens.to).toHaveBeenCalledTimes(1);
  });

  it('closes straight away when there is no body to animate', () => {
    const open = ref<boolean>(true);
    useModal(open, ref<HTMLElement | null>(null)).requestClose();
    expect(open.value).toBe(false);
    expect(tweens.to).not.toHaveBeenCalled();
  });
});
