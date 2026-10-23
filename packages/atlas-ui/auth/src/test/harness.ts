import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import { vi, type Mock } from 'vitest';
import type { Component } from 'vue';
import type { ApiResult } from '@atlas/api';
import { AUTH_CLIENT_KEY, type AuthClient } from '../client';

export type RequestMock = Mock<
  (
    path: string,
    init?: { method?: string; body?: unknown },
  ) => Promise<ApiResult<unknown>>
>;

export function ok<T>(data: T, message = 'OK', status = 200): ApiResult<T> {
  return { status, body: { success: true, message, data } };
}

export function fail(
  status: number,
  message: string,
  extra: {
    code?: string;
    data?: { tenants?: { id: string; name: string }[] };
  } = {},
): ApiResult<never> {
  return { status, body: { success: false, message, ...extra } };
}

export function mountWithClient(
  component: Component,
  request: RequestMock,
  options: {
    props?: Record<string, unknown>;
    slots?: Record<string, string>;
  } = {},
): VueWrapper {
  const client: AuthClient = {
    request: request as unknown as AuthClient['request'],
  };
  return mount(component, {
    props: options.props,
    slots: options.slots,
    global: { provide: { [AUTH_CLIENT_KEY as symbol]: client } },
  });
}

export function requestMock(): RequestMock {
  return vi.fn() as RequestMock;
}

// vee-validate schedules validation on a timer, which flushPromises alone doesn't wait for
export async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) {
    await new Promise<void>((resolve) => setTimeout(resolve, 0));
    await flushPromises();
  }
}

export async function fill(
  wrapper: VueWrapper,
  selector: string,
  value: string,
): Promise<void> {
  await wrapper.find(selector).setValue(value);
}
