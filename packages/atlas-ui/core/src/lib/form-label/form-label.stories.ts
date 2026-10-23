import type { Meta, StoryObj } from '@storybook/vue3-vite';
import FormLabel from './form-label.vue';

const meta = {
  title: 'UI/FormLabel',
  component: FormLabel,
  tags: ['autodocs'],
  argTypes: {
    disabled: { control: 'boolean' },
    htmlFor: { control: 'text' },
  },
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof FormLabel>;

export default meta;
type Story = StoryObj<typeof FormLabel>;

export const Default: Story = {
  render: (args) => ({
    components: { FormLabel },
    setup: () => ({ args }),
    template: `<FormLabel v-bind="args">Full Name</FormLabel>`,
  }),
  args: {
    htmlFor: 'fullName',
  },
};

export const WithInputExample: Story = {
  render: () => ({
    components: { FormLabel },
    template: `
      <div class="flex flex-col gap-1.5">
        <FormLabel html-for="email">Email Address</FormLabel>
        <input
          id="email"
          type="email"
          placeholder="you@example.com"
          class="border-border-soft rounded-lg border px-4 py-2"
        />
      </div>
    `,
  }),
};

export const Disabled: Story = {
  render: (args) => ({
    components: { FormLabel },
    setup: () => ({ args }),
    template: `<FormLabel v-bind="args">Date of Birth</FormLabel>`,
  }),
  args: {
    disabled: true,
  },
};

export const CustomClass: Story = {
  render: (args) => ({
    components: { FormLabel },
    setup: () => ({ args }),
    template: `<FormLabel v-bind="args" class="text-lg font-semibold text-ink-primary">Phone Number</FormLabel>`,
  }),
};

export const RequiredLabel: Story = {
  render: () => ({
    components: { FormLabel },
    template: `
      <div class="flex items-center gap-1.5">
        <FormLabel html-for="password">Password</FormLabel>
        <span class="text-sm text-red-500">*</span>
      </div>
    `,
  }),
};
