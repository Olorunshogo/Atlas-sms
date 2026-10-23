import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  fail,
  fill,
  mountWithClient,
  ok,
  requestMock,
  settle,
} from '../test/harness';
import VerifyEmailForm from './verify-email-form.vue';

afterEach(() => {
  vi.useRealTimers();
});

describe('VerifyEmailForm', () => {
  it('verifies the known email without asking for it again', async () => {
    const request = requestMock().mockResolvedValue(
      ok(null, 'Email verified.'),
    );
    const wrapper = mountWithClient(VerifyEmailForm, request, {
      props: { email: 'ada@example.com' },
    });
    expect(wrapper.find('input[type="email"]').exists()).toBe(false);

    await fill(wrapper, 'input[inputmode="numeric"]', '123456');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).toHaveBeenCalledWith('auth/verify-email', {
      method: 'POST',
      body: { email: 'ada@example.com', code: '123456' },
    });
    expect(wrapper.emitted('verified')?.[0]).toEqual(['ada@example.com']);
  });

  it('rejects a code that is not six digits', async () => {
    const request = requestMock();
    const wrapper = mountWithClient(VerifyEmailForm, request, {
      props: { email: 'ada@example.com' },
    });
    await fill(wrapper, 'input[inputmode="numeric"]', '12ab');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Enter the 6-digit code');
  });

  it('asks for the email when none is known', async () => {
    const request = requestMock().mockResolvedValue(ok(null));
    const wrapper = mountWithClient(VerifyEmailForm, request);
    await fill(wrapper, 'input[type="email"]', 'ada@example.com');
    await fill(wrapper, 'input[inputmode="numeric"]', '654321');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request.mock.calls[0]?.[1]).toEqual({
      method: 'POST',
      body: { email: 'ada@example.com', code: '654321' },
    });
  });

  it('shows the error for a wrong code and does not emit', async () => {
    const request = requestMock().mockResolvedValue(
      fail(400, 'Invalid or expired code.'),
    );
    const wrapper = mountWithClient(VerifyEmailForm, request, {
      props: { email: 'ada@example.com' },
    });
    await fill(wrapper, 'input[inputmode="numeric"]', '000000');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(wrapper.text()).toContain('Invalid or expired code.');
    expect(wrapper.emitted('verified')).toBeUndefined();
  });

  it('resends a code, then waits out the cooldown before allowing another', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const request = requestMock().mockResolvedValue(
      ok(
        null,
        'If that account still needs verifying, a new code is on its way.',
      ),
    );
    const wrapper = mountWithClient(VerifyEmailForm, request, {
      props: { email: 'ada@example.com' },
    });

    await wrapper.find('[data-test="resend"]').trigger('click');
    await settle();
    expect(request).toHaveBeenCalledWith('auth/resend-verification', {
      method: 'POST',
      body: { email: 'ada@example.com' },
    });
    expect(wrapper.text()).toContain('a new code is on its way');
    const resend = wrapper.find('[data-test="resend"]');
    expect(resend.attributes('disabled')).toBeDefined();
    expect(resend.text()).toBe('Send a new code in 60s');

    vi.advanceTimersByTime(60_000);
    await settle();
    expect(
      wrapper.find('[data-test="resend"]').attributes('disabled'),
    ).toBeUndefined();
    expect(wrapper.find('[data-test="resend"]').text()).toBe('Send a new code');
  });

  it('starts the cooldown when the server says it is too soon', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const request = requestMock().mockResolvedValue(
      fail(
        429,
        'A code was just sent. Wait a minute before asking for another.',
      ),
    );
    const wrapper = mountWithClient(VerifyEmailForm, request, {
      props: { email: 'ada@example.com' },
    });

    await wrapper.find('[data-test="resend"]').trigger('click');
    await settle();
    expect(wrapper.text()).toContain('Wait a minute');
    expect(
      wrapper.find('[data-test="resend"]').attributes('disabled'),
    ).toBeDefined();
  });
});
