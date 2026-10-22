import type { Meta, StoryObj } from '@storybook/vue3-vite';
import DashboardFrame from './dashboard-frame.vue';

const meta = {
  component: DashboardFrame,
  title: 'Web/DashboardFrame',
  tags: ['autodocs'],
} satisfies Meta<typeof DashboardFrame>;

export default meta;
type Story = StoryObj<typeof DashboardFrame>;

export const Default: Story = {
  args: {
    storeName: 'Main store',
  },
};
