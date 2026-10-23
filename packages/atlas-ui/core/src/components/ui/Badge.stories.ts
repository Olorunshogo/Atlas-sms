import type { Meta, StoryObj } from '@storybook/vue3-vite';
import Badge from './Badge.vue';

const meta = {
  component: Badge,
  title: 'Core/Badge',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof Badge>;

export default meta;
type Story = StoryObj<typeof Badge>;

export const InStock: Story = {
  render: () => ({
    components: { Badge },
    template: '<Badge variant="success">In stock</Badge>',
  }),
};

export const LowStock: Story = {
  render: () => ({
    components: { Badge },
    template: '<Badge variant="warning">Low stock</Badge>',
  }),
};

export const OutOfStock: Story = {
  render: () => ({
    components: { Badge },
    template: '<Badge variant="destructive">Out of stock</Badge>',
  }),
};

export const Neutral: Story = {
  render: () => ({
    components: { Badge },
    template: '<Badge variant="neutral">Not tracked</Badge>',
  }),
};
