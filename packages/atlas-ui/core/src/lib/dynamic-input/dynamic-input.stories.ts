import type { Meta, StoryObj } from '@storybook/vue3-vite';
import DynamicInput from './dynamic-input.vue';

const meta = {
  component: DynamicInput,
  title: 'Core/DynamicInput',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof DynamicInput>;

export default meta;
type Story = StoryObj<typeof DynamicInput>;

export const Default: Story = {
  args: {
    fieldType: 'text',
    label: 'First name',
    placeholder: 'Enter text...',
  },
};
