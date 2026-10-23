import { defineComponent, h } from 'vue';
import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import AtlasDialogProvider from './dialog-provider.vue';
import DialogBody from './dialog-body.vue';
import DialogHeader from './dialog-header.vue';
import { useAtlasDialog } from './use-atlas-dialog';

/**
 * A demo dialog registered like any real one would be - a component that
 * reads `useAtlasDialog()` for its own close button via DialogHeader.
 */
const DemoDialog = defineComponent({
  props: { title: { type: String, default: 'Example dialog' } },
  setup(props) {
    return () =>
      h(DialogBody, null, {
        default: () => [
          h(DialogHeader, null, { default: () => props.title }),
          h(
            'p',
            { class: 'text-body text-ink-secondary' },
            'This is rendered from the dialog registry, lazy-loaded on trigger.',
          ),
        ],
      });
  },
});

const meta = {
  component: AtlasDialogProvider,
  title: 'Dialog/AtlasDialogProvider',
  tags: ['autodocs'],
} satisfies Meta<typeof AtlasDialogProvider>;
export default meta;

type Story = StoryObj<typeof AtlasDialogProvider>;

const Trigger = defineComponent({
  setup() {
    const { triggerDialog } = useAtlasDialog();
    return () =>
      h(
        'button',
        {
          class: 'rounded-md bg-ink-primary px-4 py-2 text-ink-inverse',
          onClick: () => triggerDialog('DEMO', { title: 'Example dialog' }),
        },
        'Open dialog',
      );
  },
});

export const Default: Story = {
  render: () => ({
    components: { AtlasDialogProvider, Trigger },
    template: `
      <AtlasDialogProvider :registry="{ DEMO: () => Promise.resolve({ default: DemoDialog }) }">
        <Trigger />
      </AtlasDialogProvider>
    `,
    setup: () => ({ DemoDialog }),
  }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByText('Open dialog'));
    await waitFor(() =>
      expect(within(document.body).getByText('Example dialog')).toBeTruthy(),
    );
  },
};
