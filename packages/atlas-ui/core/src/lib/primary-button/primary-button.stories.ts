import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ArrowRight } from 'lucide-vue-next';
import PrimaryButton from './primary-button.vue';

const meta = {
  component: PrimaryButton,
  title: 'Core/PrimaryButton',
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    icon: { control: false },
  },
  parameters: { layout: 'centered' },
} satisfies Meta<typeof PrimaryButton>;

export default meta;
type Story = StoryObj<typeof PrimaryButton>;

export const Default: Story = {
  args: {
    label: 'Get Started',
  },
};

export const WithIcon: Story = {
  args: {
    label: 'Continue',
    icon: ArrowRight,
  },
};
