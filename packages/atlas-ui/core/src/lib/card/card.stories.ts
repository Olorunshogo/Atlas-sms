import type { Meta, StoryObj } from '@storybook/vue3-vite';
import Card from './card.vue';

const meta = {
  component: Card,
  title: 'Core/Card',
  tags: ['autodocs'],
} satisfies Meta<typeof Card>;

export default meta;
type Story = StoryObj<typeof Card>;

export const Default: Story = {
  render: () => ({
    components: { Card },
    template: '<Card><p>Store management</p></Card>',
  }),
};
