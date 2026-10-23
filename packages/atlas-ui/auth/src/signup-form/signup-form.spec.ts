import { describe, expect, it } from 'vitest';
import {
  fail,
  fill,
  mountWithClient,
  ok,
  requestMock,
  settle,
} from '../test/harness';
import SignupForm from './signup-form.vue';

async function fillAll(
  wrapper: ReturnType<typeof mountWithClient>,
): Promise<void> {
  await fill(wrapper, 'input[autocomplete="name"]', 'Ada Obi');
  await fill(wrapper, 'input[autocomplete="organization"]', 'Ada Foods');
  await fill(wrapper, 'input[type="email"]', 'ada@example.com');
  await fill(wrapper, 'input[type="tel"]', '+2348031234567');
  await fill(wrapper, 'input[type="password"]', 'long-enough-password');
}

describe('SignupForm', () => {
  it('creates the business and emits the email to verify', async () => {
    const request = requestMock().mockResolvedValue(
      ok({ email: 'ada@example.com' }, 'Registered', 201),
    );
    const wrapper = mountWithClient(SignupForm, request);
    await fillAll(wrapper);
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).toHaveBeenCalledWith('auth/signup', {
      method: 'POST',
      body: {
        full_name: 'Ada Obi',
        tenant_name: 'Ada Foods',
        email: 'ada@example.com',
        phone: '+2348031234567',
        password: 'long-enough-password',
      },
    });
    expect(wrapper.emitted('success')?.[0]).toEqual(['ada@example.com']);
  });

  it('refuses a short password before calling the API', async () => {
    const request = requestMock();
    const wrapper = mountWithClient(SignupForm, request);
    await fillAll(wrapper);
    await fill(wrapper, 'input[type="password"]', 'short');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Password must be at least 8 characters');
  });

  it('shows the server error for an email already in use', async () => {
    const request = requestMock().mockResolvedValue(
      fail(400, 'This email is already registered. Please log in.', {
        code: 'EMAIL_ALREADY_REGISTERED',
      }),
    );
    const wrapper = mountWithClient(SignupForm, request);
    await fillAll(wrapper);
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(wrapper.text()).toContain('already registered');
    expect(wrapper.emitted('success')).toBeUndefined();
  });
});
