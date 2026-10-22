import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ArrowRight, ExternalLink } from 'lucide-vue-next';
import PrimaryLink from './primary-link.vue';

const meta = {
  component: PrimaryLink,
  title: 'Core/PrimaryLink',
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    href: { control: 'text' },
    icon: { control: false },
  },
  parameters: { layout: 'centered' },
} satisfies Meta<typeof PrimaryLink>;

export default meta;
type Story = StoryObj<typeof PrimaryLink>;

export const Default: Story = {
  args: {
    label: 'Get Started',
    href: '/signup',
  },
};

export const WithIcon: Story = {
  args: {
    label: 'Continue',
    href: '/dashboard',
    icon: ArrowRight,
  },
};

export const ExternalLinkExample: Story = {
  args: {
    label: 'Visit Site',
    href: 'https://example.com',
    icon: ExternalLink,
  },
};
