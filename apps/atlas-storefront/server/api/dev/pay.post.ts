import { createHash } from 'node:crypto';

/**
 * Development only: plays the mock payment gateway's server-to-server callback, signed the way the
 * backend's mock gateway expects (hex SHA-256 of secret then body). Off unless mock integrations are on
 * and the webhook secret is configured, so it can't run against a real gateway.
 */
export default defineEventHandler(async (event): Promise<unknown> => {
  const config = useRuntimeConfig();
  if (!config.public.mockIntegrations || !config.webhookSecret) {
    setResponseStatus(event, 404);
    return { success: false, message: 'Mock payments are off.' };
  }
  const input = await readBody<{ gateway_ref?: string; status?: string }>(
    event,
  );
  if (
    !input?.gateway_ref ||
    (input.status !== 'succeeded' && input.status !== 'failed')
  ) {
    setResponseStatus(event, 400);
    return {
      success: false,
      message: 'gateway_ref and status (succeeded or failed) are required.',
    };
  }
  const body = JSON.stringify({
    gateway_ref: input.gateway_ref,
    status: input.status,
  });
  const signature = createHash('sha256')
    .update(config.webhookSecret)
    .update(body)
    .digest('hex');
  const response = await $fetch.raw(
    `${config.apiBase.replace(/\/$/, '')}/api/v1/shop-webhooks/payments`,
    {
      method: 'POST',
      body,
      headers: {
        'content-type': 'application/json',
        'x-gateway-signature': signature,
      },
      ignoreResponseError: true,
    },
  );
  setResponseStatus(event, response.status);
  return response._data;
});
