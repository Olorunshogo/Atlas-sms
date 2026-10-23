import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ref } from 'vue';
import SelectInput from './select-input.vue';

const meta = {
  title: 'Core/SelectInput',
  component: SelectInput,
  tags: ['autodocs'],
  argTypes: {
    placeholder: { control: 'text' },
    disabled: { control: 'boolean' },
  },
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof SelectInput>;

export default meta;
type Story = StoryObj<typeof SelectInput>;

const sampleOptions = [
  { value: 'lagos', label: 'Lagos' },
  { value: 'abuja', label: 'Abuja (FCT)' },
  { value: 'rivers', label: 'Rivers' },
  { value: 'kaduna', label: 'Kaduna' },
  { value: 'oyo', label: 'Oyo' },
];

export const Default: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'State',
    placeholder: 'Select state',
    options: sampleOptions,
  },
};

export const WithValue: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('lagos') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'State',
    placeholder: 'Select state',
    options: sampleOptions,
  },
};

export const Disabled: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'Area',
    placeholder: 'Select area',
    options: sampleOptions,
    disabled: true,
  },
};

export const Searchable: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'State',
    placeholder: 'Select state',
    options: sampleOptions,
    searchable: true,
  },
};

export const WithError: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'State',
    placeholder: 'Select state',
    options: sampleOptions,
    error: true,
    errorMessage: 'State is required.',
  },
};

export const LongPlaceholderDisabled: Story = {
  render: (args) => ({
    components: { SelectInput },
    setup: () => ({ args, value: ref('') }),
    template: `<SelectInput v-bind="args" v-model="value" />`,
  }),
  args: {
    label: 'Area',
    placeholder: 'First select a state to see available areas',
    options: [],
    disabled: true,
  },
};
