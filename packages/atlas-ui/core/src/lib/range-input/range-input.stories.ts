import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ref } from 'vue';
import RangeInput from './range-input.vue';

const meta = {
  title: 'Core/PriceRangeSlider',
  component: RangeInput,
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    min: { control: { type: 'number' } },
    max: { control: { type: 'number' } },
    step: { control: { type: 'number', step: 10000 } },
    disabled: { control: 'boolean' },
  },
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof RangeInput>;

export default meta;
type Story = StoryObj<typeof RangeInput>;

function interactive(
  args: Story['args'],
  initial: { min: number; max: number },
) {
  return {
    args,
    render: (renderArgs: Story['args']) => ({
      components: { RangeInput },
      setup: () => ({ args: renderArgs, value: ref(initial) }),
      template: `<RangeInput v-bind="args" v-model="value" />`,
    }),
  };
}

export const Default: Story = interactive(
  { label: 'Price Range', min: 700_000, max: 15_000_000, step: 50_000 },
  { min: 700_000, max: 3_500_000 },
);

export const HighRange: Story = interactive(
  {
    label: 'Price Range (Premium)',
    min: 3_000_000,
    max: 20_000_000,
    step: 100_000,
  },
  { min: 5_000_000, max: 12_000_000 },
);

export const Disabled: Story = interactive(
  { label: 'Price Range', disabled: true },
  { min: 1_000_000, max: 4_000_000 },
);

export const WithCustomLabel: Story = interactive(
  { label: 'Annual Rent Range' },
  { min: 800_000, max: 2_800_000 },
);
