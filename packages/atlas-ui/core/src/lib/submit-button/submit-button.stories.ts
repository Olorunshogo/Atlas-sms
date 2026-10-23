import type { Meta, StoryObj } from '@storybook/vue3-vite';
import SubmitButton from './submit-button.vue';

const meta = {
  component: SubmitButton,
  title: 'Core/SubmitButton',
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof SubmitButton>;

export default meta;
type Story = StoryObj<typeof SubmitButton>;

export const BrandSolid: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Join Waitlist</SubmitButton>`,
  }),
  args: { color: 'brand', variant: 'solid' },
};

export const NeutralSolid: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Save Changes</SubmitButton>`,
  }),
  args: { color: 'neutral', variant: 'solid' },
};

export const DangerSolid: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Delete Account</SubmitButton>`,
  }),
  args: { color: 'danger', variant: 'solid' },
};

export const BrandOutline: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Secondary Action</SubmitButton>`,
  }),
  args: { color: 'brand', variant: 'outline' },
};

export const NeutralOutline: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Cancel</SubmitButton>`,
  }),
  args: { color: 'neutral', variant: 'outline' },
};

export const DangerOutline: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Reject Application</SubmitButton>`,
  }),
  args: { color: 'danger', variant: 'outline' },
};

export const Loading: Story = {
  render: (args) => ({
    components: { SubmitButton },
    setup: () => ({ args }),
    template: `<SubmitButton v-bind="args">Joining...</SubmitButton>`,
  }),
  args: {
    color: 'brand',
    variant: 'solid',
    loading: true,
    loadingText: 'Joining...',
  },
};
