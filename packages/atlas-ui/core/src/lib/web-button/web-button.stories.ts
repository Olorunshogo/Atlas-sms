import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { expect, within } from 'storybook/test';
import WebButton from './web-button.vue';

const meta = {
  component: WebButton,
  title: 'Core/Button',
  tags: ['autodocs'],
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof WebButton>;
export default meta;

type Story = StoryObj<typeof WebButton>;

export const Default: Story = {
  render: () => ({
    components: { WebButton },
    template: `<WebButton>Default Button</WebButton>`,
  }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(/Default Button/i)).toBeTruthy();
  },
};

export const Disabled: Story = {
  render: () => ({
    components: { WebButton },
    template: `<WebButton disabled>Disabled Button</WebButton>`,
  }),
};

export const IconOnly: Story = {
  render: () => ({
    components: { WebButton },
    template: `<WebButton />`,
  }),
};
