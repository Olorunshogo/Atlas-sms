import type { Meta, StoryObj } from '@storybook/vue3-vite';
import CloseButton from './close-button.vue';

const meta = {
  component: CloseButton,
  title: 'Core/CloseButton',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof CloseButton>;

export default meta;
type Story = StoryObj<typeof CloseButton>;

export const Default: Story = {
  args: {},
};
