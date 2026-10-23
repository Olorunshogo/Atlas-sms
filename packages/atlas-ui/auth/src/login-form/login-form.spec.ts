import { describe, expect, it } from 'vitest';
import {
  fail,
  fill,
  mountWithClient,
  ok,
  requestMock,
  settle,
} from '../test/harness';
import LoginForm from './login-form.vue';

const session = {
  tenant_id: 't1',
  user: {
    id: 'u1',
    full_name: 'Ada',
    email: 'ada@example.com',
    email_verified: false,
  },
};

describe('LoginForm', () => {
  it('logs in and emits the session, unverified accounts included', async () => {
    const request = requestMock().mockResolvedValue(ok(session));
    const wrapper = mountWithClient(LoginForm, request, {
      props: { initialEmail: 'ada@example.com' },
    });
    await fill(wrapper, 'input[type="password"]', 'secret-pass');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).toHaveBeenCalledWith('auth/login', {
      method: 'POST',
      body: { email: 'ada@example.com', password: 'secret-pass' },
    });
    expect(wrapper.emitted('success')?.[0]).toEqual([session]);
  });

  it('does not call the API when fields are invalid', async () => {
    const request = requestMock();
    const wrapper = mountWithClient(LoginForm, request);
    await fill(wrapper, 'input[type="email"]', 'not-an-email');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(request).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Enter a valid email');
    expect(wrapper.text()).toContain('Password is required');
  });

  it('asks for a business when the account belongs to several, then sends it', async () => {
    const tenants = [
      { id: 't1', name: 'Lagos Foods' },
      { id: 't2', name: 'Abuja Mart' },
    ];
    const request = requestMock()
      .mockResolvedValueOnce(
        fail(409, 'Choose a business.', {
          code: 'TENANT_REQUIRED',
          data: { tenants },
        }),
      )
      .mockResolvedValueOnce(ok(session));
    const wrapper = mountWithClient(LoginForm, request);
    await fill(wrapper, 'input[type="email"]', 'ada@example.com');
    await fill(wrapper, 'input[type="password"]', 'secret-pass');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(wrapper.text()).toContain('Choose a business.');
    const options = wrapper.findAll('option').map((o) => o.text());
    expect(options).toEqual(['Choose a business', 'Lagos Foods', 'Abuja Mart']);

    await wrapper.find('select').setValue('t2');
    await wrapper.find('form').trigger('submit');
    await settle();
    expect(request.mock.calls[1]?.[1]).toEqual({
      method: 'POST',
      body: {
        email: 'ada@example.com',
        password: 'secret-pass',
        tenant_id: 't2',
      },
    });
    expect(wrapper.emitted('success')).toHaveLength(1);
  });

  it('shows the server message on a wrong password', async () => {
    const request = requestMock().mockResolvedValue(
      fail(400, 'Wrong email or password.'),
    );
    const wrapper = mountWithClient(LoginForm, request);
    await fill(wrapper, 'input[type="email"]', 'ada@example.com');
    await fill(wrapper, 'input[type="password"]', 'nope');
    await wrapper.find('form').trigger('submit');
    await settle();

    expect(wrapper.find('[role="alert"]').text()).toBe(
      'Wrong email or password.',
    );
    expect(wrapper.emitted('success')).toBeUndefined();
  });
});
