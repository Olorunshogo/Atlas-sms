import { describe, expect, it } from 'vitest';
import { AcceptInviteForm } from './accept-invite-form';
import { ForgotPasswordForm } from './forgot-password-form';
import { ResetPasswordForm } from './reset-password-form';
import {
  fail,
  fill,
  mountWithClient,
  ok,
  requestMock,
  settle,
} from './test/harness';

describe('ForgotPasswordForm', () => {
  it('requests a code and emits the email', async () => {
    const request = requestMock().mockResolvedValue(ok(null));
    const wrapper = mountWithClient(ForgotPasswordForm, request);
    await fill(wrapper, 'input[type="email"]', 'ada@example.com');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).toHaveBeenCalledWith('auth/forgot-password', {
      method: 'POST',
      body: { email: 'ada@example.com' },
    });
    expect(wrapper.emitted('sent')?.[0]).toEqual(['ada@example.com']);
  });
});

describe('ResetPasswordForm', () => {
  it('resets with the code and new password', async () => {
    const request = requestMock().mockResolvedValue(ok(null));
    const wrapper = mountWithClient(ResetPasswordForm, request, {
      props: { initialEmail: 'ada@example.com' },
    });
    await fill(wrapper, 'input[inputmode="numeric"]', '123456');
    await fill(wrapper, 'input[type="password"]', 'a-new-long-password');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).toHaveBeenCalledWith('auth/reset-password', {
      method: 'POST',
      body: {
        email: 'ada@example.com',
        code: '123456',
        new_password: 'a-new-long-password',
      },
    });
    expect(wrapper.emitted('reset')?.[0]).toEqual(['ada@example.com']);
  });
});

describe('AcceptInviteForm', () => {
  it('accepts against the invite token', async () => {
    const request = requestMock().mockResolvedValue(ok(null));
    const wrapper = mountWithClient(AcceptInviteForm, request, {
      props: { token: 'tok/en' },
    });
    await fill(wrapper, 'input[autocomplete="name"]', 'Bola Ade');
    await fill(wrapper, 'input[type="tel"]', '+2348030000000');
    await fill(wrapper, 'input[type="password"]', 'long-enough-password');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request.mock.calls[0]?.[0]).toBe('auth/invites/tok%2Fen/accept');
    expect(wrapper.emitted('accepted')).toHaveLength(1);
  });

  it('shows an expired invite error', async () => {
    const request = requestMock().mockResolvedValue(
      fail(400, 'This invite has expired.'),
    );
    const wrapper = mountWithClient(AcceptInviteForm, request, {
      props: { token: 'old' },
    });
    await fill(wrapper, 'input[autocomplete="name"]', 'Bola Ade');
    await fill(wrapper, 'input[type="tel"]', '+2348030000000');
    await fill(wrapper, 'input[type="password"]', 'long-enough-password');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(wrapper.text()).toContain('This invite has expired.');
  });
});
