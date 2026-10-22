import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ArrowRight, ExternalLink } from 'lucide-vue-next';
import SecondaryLink from './secondary-link.vue';

const meta = {
  component: SecondaryLink,
  title: 'Core/SecondaryLink',
  tags: ['autodocs'],
  argTypes: {
    label: { control: 'text' },
    href: { control: 'text' },
    icon: { control: false },
  },
  parameters: { layout: 'centered' },
} satisfies Meta<typeof SecondaryLink>;

export default meta;
type Story = StoryObj<typeof SecondaryLink>;

export const Default: Story = {
  args: {
    label: 'Learn More',
    href: '/about',
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
