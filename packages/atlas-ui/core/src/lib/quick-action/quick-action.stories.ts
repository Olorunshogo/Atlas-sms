import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { PackagePlus } from 'lucide-vue-next';
import QuickAction from './quick-action.vue';

const meta = {
  component: QuickAction,
  title: 'Core/QuickAction',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof QuickAction>;

export default meta;
type Story = StoryObj<typeof QuickAction>;

export const Default: Story = {
  args: {
    label: 'Add product',
    icon: PackagePlus,
  },
};
