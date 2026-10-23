import { describe, expect, it } from 'vitest';
import {
  fill,
  mountWithClient,
  ok,
  requestMock,
  settle,
} from '../test/harness';
import EmailVerificationGate from './email-verification-gate.vue';

const slots = { default: '<p data-test="dashboard">Dashboard content</p>' };

describe('EmailVerificationGate', () => {
  it('renders the page as-is for a verified account', () => {
    const wrapper = mountWithClient(EmailVerificationGate, requestMock(), {
      props: { verified: true, email: 'ada@example.com' },
      slots,
    });
    expect(wrapper.find('[data-test="dashboard"]').exists()).toBe(true);
    expect(wrapper.find('[data-test="verification-gate"]').exists()).toBe(
      false,
    );
  });

  it('holds the page back and shows the banner and code form until verified', async () => {
    const request = requestMock().mockResolvedValue(ok(null));
    const wrapper = mountWithClient(EmailVerificationGate, request, {
      props: { verified: false, email: 'ada@example.com' },
      slots,
    });
    expect(wrapper.find('[data-test="dashboard"]').exists()).toBe(false);
    expect(wrapper.find('[role="status"]').text()).toContain('ada@example.com');

    await fill(wrapper, 'input[inputmode="numeric"]', '123456');
    await wrapper.find('form').trigger('submit');
    await settle();
    expect(wrapper.emitted('verified')).toHaveLength(1);

    await wrapper.setProps({ verified: true });
    expect(wrapper.find('[data-test="dashboard"]').exists()).toBe(true);
  });
});
