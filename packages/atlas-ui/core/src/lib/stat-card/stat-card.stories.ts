import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { Banknote } from 'lucide-vue-next';
import StatCard from './stat-card.vue';

const meta = {
  component: StatCard,
  title: 'Core/StatCard',
  tags: ['autodocs'],
} satisfies Meta<typeof StatCard>;

export default meta;
type Story = StoryObj<typeof StatCard>;

export const AwaitingData: Story = {
  args: {
    label: "Today's sales",
    value: '\u2014',
    sub: 'Awaiting live data',
    icon: Banknote,
  },
};
