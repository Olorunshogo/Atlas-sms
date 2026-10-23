import type { Meta, StoryObj } from '@storybook/vue3-vite';
import SaleSession from './sale-session.vue';

const meta = {
  component: SaleSession,
  title: 'App/SaleSession',
  tags: ['autodocs'],
} satisfies Meta<typeof SaleSession>;

export default meta;
type Story = StoryObj<typeof SaleSession>;

export const EmptyCart: Story = {
  args: {
    storeName: 'Main store',
    online: true,
  },
};
