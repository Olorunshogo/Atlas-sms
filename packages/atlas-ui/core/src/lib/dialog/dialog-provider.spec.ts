import { describe, expect, it } from 'vitest';
import { defineComponent, h, inject } from 'vue';
import { flushPromises, mount } from '@vue/test-utils';
import AtlasDialogProvider from './dialog-provider.vue';
import { ATLAS_DIALOG_KEY } from './use-atlas-dialog';
import type { DialogRegistry, AtlasDialogContext } from './dialog-types';

const DialogA = defineComponent({
  name: 'DialogA',
  render: () => h('div', { 'data-testid': 'dialog-a' }, 'A'),
});

const DialogB = defineComponent({
  name: 'DialogB',
  render: () => h('div', { 'data-testid': 'dialog-b' }, 'B'),
});

const registry: DialogRegistry = {
  DIALOG_A: () => Promise.resolve({ default: DialogA }),
  DIALOG_B: () => Promise.resolve({ default: DialogB }),
};

/**
 * Host component exposes the injected context via a template ref so tests
 * can drive triggerDialog/closeDialog without reaching into internals.
 */
function mountHost() {
  let context: AtlasDialogContext | undefined;

  const Host = defineComponent({
    setup() {
      context = inject(ATLAS_DIALOG_KEY);
      return () => h('div');
    },
  });

  const wrapper = mount(AtlasDialogProvider, {
    props: { registry },
    slots: { default: () => h(Host) },
    attachTo: document.body,
  });

  if (!context)
    throw new Error('AtlasDialogProvider did not provide a context');
  return { wrapper, context };
}

describe('AtlasDialogProvider', () => {
  it('throws when triggering a key not in the registry', () => {
    const { context } = mountHost();
    expect(() => context.triggerDialog('NOT_REGISTERED')).toThrow(
      /not in the registry/i,
    );
  });

  it('opens a dialog registered by key', async () => {
    const { wrapper, context } = mountHost();
    context.triggerDialog('DIALOG_A');
    await wrapper.vm.$nextTick();
    await flushPromises();
    expect(
      document.body.querySelector('[data-testid="dialog-a"]'),
    ).not.toBeNull();
  });

  it('replaces the stack by default, but stacks with keepParent', async () => {
    const { wrapper, context } = mountHost();

    context.triggerDialog('DIALOG_A');
    await wrapper.vm.$nextTick();
    context.triggerDialog('DIALOG_B');
    await wrapper.vm.$nextTick();
    await flushPromises();

    expect(document.body.querySelector('[data-testid="dialog-a"]')).toBeNull();
    expect(
      document.body.querySelector('[data-testid="dialog-b"]'),
    ).not.toBeNull();

    context.triggerDialog('DIALOG_A', undefined, { keepParent: true });
    await wrapper.vm.$nextTick();
    await flushPromises();

    expect(
      document.body.querySelector('[data-testid="dialog-a"]'),
    ).not.toBeNull();
    expect(
      document.body.querySelector('[data-testid="dialog-b"]'),
    ).not.toBeNull();
  });

  it('closeDialog() closes only the top of the stack', async () => {
    const { wrapper, context } = mountHost();

    context.triggerDialog('DIALOG_A');
    await wrapper.vm.$nextTick();
    context.triggerDialog('DIALOG_B', undefined, { keepParent: true });
    await wrapper.vm.$nextTick();
    await flushPromises();

    context.closeDialog();
    await wrapper.vm.$nextTick();
    await flushPromises();

    expect(document.body.querySelector('[data-testid="dialog-b"]')).toBeNull();
    expect(
      document.body.querySelector('[data-testid="dialog-a"]'),
    ).not.toBeNull();
  });

  it('closeDialog({ closeParent: true }) closes the whole stack', async () => {
    const { wrapper, context } = mountHost();

    context.triggerDialog('DIALOG_A');
    await wrapper.vm.$nextTick();
    context.triggerDialog('DIALOG_B', undefined, { keepParent: true });
    await wrapper.vm.$nextTick();
    await flushPromises();

    context.closeDialog(undefined, { closeParent: true });
    await wrapper.vm.$nextTick();
    await flushPromises();

    expect(document.body.querySelector('[data-testid="dialog-a"]')).toBeNull();
    expect(document.body.querySelector('[data-testid="dialog-b"]')).toBeNull();
  });

  it('presentLoading/closeLoading toggles the loading overlay independently of the stack', async () => {
    const { wrapper, context } = mountHost();

    context.triggerDialog('DIALOG_A');
    await wrapper.vm.$nextTick();

    context.presentLoading('Saving...');
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).toContain('Saving...');

    context.closeLoading();
    await wrapper.vm.$nextTick();
    expect(document.body.textContent).not.toContain('Saving...');
  });
});
