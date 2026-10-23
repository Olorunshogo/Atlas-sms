import type { Meta, StoryObj } from '@storybook/vue3-vite';
import { ref } from 'vue';
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSeparator,
  InputOTPSlot,
} from '../../components/ui/input-otp';

const meta = {
  title: 'Core/InputOTP',
  component: InputOTP,
  tags: ['autodocs'],
  parameters: { layout: 'centered' },
} satisfies Meta<typeof InputOTP>;

export default meta;
type Story = StoryObj<typeof InputOTP>;

const otpTemplate = `
  <InputOTP v-model="value" :maxlength="6" v-slot="{ slots }">
    <InputOTPGroup>
      <InputOTPSlot v-for="(_, i) in slots.slice(0, 3)" :key="i" :index="i" />
    </InputOTPGroup>
    <InputOTPSeparator />
    <InputOTPGroup>
      <InputOTPSlot v-for="(_, i) in slots.slice(3, 6)" :key="i + 3" :index="i + 3" />
    </InputOTPGroup>
  </InputOTP>
`;

export const Default: Story = {
  render: () => ({
    components: { InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator },
    setup: () => ({ value: ref('') }),
    template: otpTemplate,
  }),
};

export const Filled: Story = {
  render: () => ({
    components: { InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator },
    setup: () => ({ value: ref('123456') }),
    template: otpTemplate,
  }),
};

export const Disabled: Story = {
  render: () => ({
    components: { InputOTP, InputOTPGroup, InputOTPSlot, InputOTPSeparator },
    setup: () => ({ value: ref('123456') }),
    template: otpTemplate.replace('<InputOTP ', '<InputOTP disabled '),
  }),
};
