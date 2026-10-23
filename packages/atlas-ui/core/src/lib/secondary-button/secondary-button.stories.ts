import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ArrowRight } from 'lucide-vue-next';
import SecondaryButton from './secondary-button.vue';

const meta = {
  component: SecondaryButton,
  title: 'Core/SecondaryButton',
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    icon: { control: false },
  },
  parameters: { layout: 'centered' },
} satisfies Meta<typeof SecondaryButton>;

export default meta;
type Story = StoryObj<typeof SecondaryButton>;

export const Default: Story = {
  args: {
    label: 'Learn More',
  },
};

export const WithIcon: Story = {
  args: {
    label: 'Continue',
    icon: ArrowRight,
  },
};
