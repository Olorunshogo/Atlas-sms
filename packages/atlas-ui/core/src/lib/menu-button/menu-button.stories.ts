import type { Meta, StoryObj } from '@storybook/vue3-vite';
import MenuButton from './menu-button.vue';

const meta = {
  component: MenuButton,
  title: 'Core/MenuButton',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof MenuButton>;

export default meta;
type Story = StoryObj<typeof MenuButton>;

export const Default: Story = {
  args: {},
};
