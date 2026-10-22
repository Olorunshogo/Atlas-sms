<script setup lang="ts">
import { toZod4TypedSchema } from '@atlas/validation';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../../utils/api';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Verify email',
  description: 'Confirm an AtlasSMS email address.',
  path: '/auth/verify-email',
  noIndex: true,
});

const route = useRoute();
const formError = ref<string>('');
const email = computed((): string => {
  return typeof route.query.email === 'string' ? route.query.email : '';
});

const schema = z.object({
  code: z
    .string()
    .min(6, 'Enter the 6-digit code')
    .max(6, 'Enter the 6-digit code'),
});

const form = useForm({
  validationSchema: toZod4TypedSchema(schema),
  initialValues: { code: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  if (!email.value) {
    formError.value = 'Missing email. Start again from signup.';
    return;
  }
  const result = await apiRequest<unknown>('v1/auth/verify-email', {
    method: 'POST',
    body: { email: email.value, code: values.code },
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  await navigateTo({ path: '/auth/login', query: { email: email.value } });
});
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <div class="mx-auto flex w-full max-w-112 flex-col gap-6">
      <h1 class="text-3xl font-medium">Verify email</h1>
      <p class="text-ink-secondary">
        Enter the code sent to {{ email || 'your email' }}. Then log in with the
        password you just set.
      </p>
      <form class="flex flex-col gap-4" @submit="onSubmit">
        <Field v-slot="{ field, errorMessage }" name="code">
          <AuthField
            label="Code"
            type="text"
            inputmode="numeric"
            autocomplete="one-time-code"
            v-bind="field"
            :error="errorMessage"
          />
        </Field>
        <p v-if="formError" class="text-danger text-sm">{{ formError }}</p>
        <button
          type="submit"
          class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
          :disabled="form.isSubmitting.value"
        >
          Verify
        </button>
      </form>
    </div>
  </main>
</template>
