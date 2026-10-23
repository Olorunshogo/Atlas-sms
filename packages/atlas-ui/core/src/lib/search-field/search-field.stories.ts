import type { Meta, StoryObj } from '@storybook/vue3-vite';
import SearchField from './search-field.vue';

const meta = {
  component: SearchField,
  title: 'Core/SearchField',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof SearchField>;

export default meta;
type Story = StoryObj<typeof SearchField>;

export const Default: Story = {
  args: {
    label: 'Find a product',
    placeholder: 'Scan barcode or search by name / SKU',
  },
};

export const Inverse: Story = {
  args: {
    placeholder: 'Scan barcode or search by name / SKU',
    tone: 'inverse',
  },
};
