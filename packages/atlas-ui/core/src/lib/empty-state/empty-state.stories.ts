import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ShoppingCart } from 'lucide-vue-next';
import EmptyState from './empty-state.vue';

const meta = {
  component: EmptyState,
  title: 'Core/EmptyState',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof EmptyState>;

export default meta;
type Story = StoryObj<typeof EmptyState>;

export const Cart: Story = {
  args: {
    title: 'Your cart is empty',
    description: 'Search or scan a product to begin the sale.',
    icon: ShoppingCart,
  },
};
