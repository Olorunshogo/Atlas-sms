import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { defineComponent, h, ref } from 'vue';
import Modal from './Modal.vue';

// Reduced motion makes every enter and exit synchronous, so these tests follow state, not tweens
function stubReducedMotion(): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({
      matches: query.includes('prefers-reduced-motion'),
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }),
  });
}

const mounted: VueWrapper[] = [];

interface MountedModal {
  wrapper: VueWrapper;
  open: ReturnType<typeof ref<boolean>>;
  afterLeave: { count: number };
}

function mountModal(blocked = false): MountedModal {
  const open = ref<boolean>(false);
  const isBlocked = ref<boolean>(blocked);
  const afterLeave = { count: 0 };
  const Host = defineComponent({
    setup() {
      return () =>
        h(
          Modal,
          {
            open: open.value,
            'onUpdate:open': (value: boolean) => {
              open.value = value;
            },
            title: 'Invite staff',
            description: 'Send an invite link.',
            blocked: isBlocked.value,
            'onAfter-leave': () => {
              afterLeave.count += 1;
            },
          },
          {
            default: ({ close }: { close: () => void }) =>
              h('div', [
                h('input', { 'data-gsap-field': '', type: 'email' }),
                h(
                  'button',
                  { 'data-test': 'cancel', onClick: close },
                  'Cancel',
                ),
              ]),
          },
        );
    },
  });
  const wrapper = mount(Host, { attachTo: document.body });
  mounted.push(wrapper);
  return { wrapper, open, afterLeave };
}

function dialog(): HTMLElement | null {
  return document.body.querySelector('[role="dialog"]');
}

beforeEach(stubReducedMotion);

afterEach(() => {
  mounted.splice(0).forEach((wrapper) => wrapper.unmount());
  document.body.innerHTML = '';
  document.body.style.overflow = '';
});

describe('Modal', () => {
  it('renders nothing until opened, then the title, description and slot', async () => {
    const { open } = mountModal();
    expect(dialog()).toBeNull();

    open.value = true;
    await flushPromises();
    expect(dialog()?.textContent).toContain('Invite staff');
    expect(dialog()?.textContent).toContain('Send an invite link.');
    expect(dialog()?.querySelector('[data-gsap-field]')).not.toBeNull();
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('closes through the slot close, then reports after-leave and unlocks the page', async () => {
    const { open, afterLeave } = mountModal();
    open.value = true;
    await flushPromises();

    dialog()?.querySelector<HTMLButtonElement>('[data-test="cancel"]')?.click();
    await flushPromises();
    expect(open.value).toBe(false);
    expect(dialog()).toBeNull();
    expect(afterLeave.count).toBe(1);
    expect(document.body.style.overflow).toBe('');
  });

  it('closes on Escape and on the backdrop', async () => {
    const { open } = mountModal();
    open.value = true;
    await flushPromises();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await flushPromises();
    expect(open.value).toBe(false);

    open.value = true;
    await flushPromises();
    document.body.querySelector<HTMLElement>('[aria-hidden="true"]')?.click();
    await flushPromises();
    expect(open.value).toBe(false);
  });

  it('cannot be closed at all while blocked', async () => {
    const { open, afterLeave } = mountModal(true);
    open.value = true;
    await flushPromises();

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    document.body.querySelector<HTMLElement>('[aria-hidden="true"]')?.click();
    dialog()?.querySelector<HTMLButtonElement>('[data-test="cancel"]')?.click();
    await flushPromises();

    expect(open.value).toBe(true);
    expect(dialog()).not.toBeNull();
    expect(dialog()?.textContent).not.toContain('Close');
    expect(afterLeave.count).toBe(0);
  });

  it('can be reopened after closing', async () => {
    const { open, afterLeave } = mountModal();
    open.value = true;
    await flushPromises();
    open.value = false;
    await flushPromises();
    open.value = true;
    await flushPromises();

    expect(dialog()).not.toBeNull();
    expect(afterLeave.count).toBe(1);
  });
});
