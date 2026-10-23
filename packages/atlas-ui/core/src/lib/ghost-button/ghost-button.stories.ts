import type { Meta, StoryObj } from '@storybook/vue3-vite';
import GhostButton from './ghost-button.vue';

const meta = {
  component: GhostButton,
  title: 'Core/GhostButton',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof GhostButton>;

export default meta;
type Story = StoryObj<typeof GhostButton>;

export const Default: Story = {
  args: {
    label: 'Cancel',
  },
};
