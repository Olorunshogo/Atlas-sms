import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ref } from 'vue';
import PriceInput from './price-input.vue';

const meta = {
  title: 'Components/PriceInput',
  component: PriceInput,
  tags: ['autodocs'],
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof PriceInput>;

export default meta;
type Story = StoryObj<typeof PriceInput>;

function interactive(initial: number) {
  return () => ({
    components: { PriceInput },
    setup: () => ({ value: ref(initial) }),
    template: `
      <div class="w-80 space-y-6 p-6">
        <div class="space-y-2">
          <label class="text-sm font-medium">Minimum Price</label>
          <PriceInput v-model="value" class="w-full" />
        </div>
      </div>
    `,
  });
}

export const Default: Story = { render: interactive(700) };
export const OnePointFiveMillion: Story = {
  render: interactive(1.5),
  name: '1.5 Million',
};
export const TwoMillion: Story = { render: interactive(2000) };
export const FiveHundred: Story = { render: interactive(500) };
export const OnePointTwoFiveMillion: Story = {
  render: interactive(1.25),
  name: '1.25 Million',
};
export const Zero: Story = { render: interactive(0) };
export const TenMillion: Story = { render: interactive(10000) };

export const MinMaxExample: Story = {
  name: 'Min-Max Example',
  render: () => ({
    components: { PriceInput },
    setup: () => ({ min: ref(500), max: ref(2500) }),
    template: `
      <div class="w-96 space-y-6 p-6">
        <div class="space-y-2">
          <label class="text-sm font-medium">Price Range</label>
          <div class="flex items-center gap-3">
            <PriceInput v-model="min" :other-value="max" is-min class="flex-1" />
            <span class="text-muted-foreground">&ndash;</span>
            <PriceInput v-model="max" :other-value="min" class="flex-1" />
          </div>
        </div>
      </div>
    `,
  }),
};
