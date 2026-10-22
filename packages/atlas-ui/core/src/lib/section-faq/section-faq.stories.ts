import type { Meta, StoryObj } from '@storybook/vue3-vite';
import SectionFAQ from './section-faq.vue';

const meta = {
  component: SectionFAQ,
  title: 'Core/SectionFAQ',
  tags: ['autodocs'],
  parameters: { layout: 'padded' },
} satisfies Meta<typeof SectionFAQ>;

export default meta;
type Story = StoryObj<typeof SectionFAQ>;

export const Default: Story = {
  args: {
    title: 'Frequently Asked Questions',
    items: [
      {
        question: 'How does Atlas handle multiple stores under one business?',
        answer:
          'Every store belongs to one tenant. Owners and admins can see and manage every store, while a store manager and cashiers only see their own store.',
      },
      {
        question: 'Can a manager see sales from another branch?',
        answer:
          'Only with a tenant-wide grant. A store-scoped role sees that store alone.',
      },
      {
        question: 'Who can invite staff?',
        answer:
          'Anyone holding staff.invite, and only for a role whose permissions they already hold.',
      },
    ],
  },
};
