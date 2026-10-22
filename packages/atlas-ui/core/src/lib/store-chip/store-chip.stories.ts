import type { Meta, StoryObj } from '@storybook/vue3-vite';
import StoreChip from './store-chip.vue';

const meta = {
  component: StoreChip,
  title: 'Core/StoreChip',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof StoreChip>;

export default meta;
type Story = StoryObj<typeof StoreChip>;

export const Default: Story = {
  args: {
    label: 'Main store',
  },
};
