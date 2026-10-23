<script setup lang="ts">
import { toZod4TypedSchema } from '@atlas/validation';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../../utils/api';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Reset password',
  description: 'Choose a new AtlasSMS password.',
  path: '/auth/reset-password',
  noIndex: true,
});

const route = useRoute();
const formError = ref<string>('');

const schema = z.object({
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
  code: z
    .string()
    .min(6, 'Enter the 6-digit code')
    .max(6, 'Enter the 6-digit code'),
  new_password: z.string().min(8, 'Password must be at least 8 characters'),
});

const form = useForm({
  validationSchema: toZod4TypedSchema(schema),
  initialValues: {
    email: typeof route.query.email === 'string' ? route.query.email : '',
    code: '',
    new_password: '',
  },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await apiRequest<unknown>('v1/auth/reset-password', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  await navigateTo({ path: '/auth/login', query: { email: values.email } });
});
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <div class="mx-auto flex w-full max-w-112 flex-col gap-6">
      <h1 class="text-3xl font-medium">Reset password</h1>
      <p class="text-ink-secondary">
        This signs you out of every device. Log in again with the new password.
      </p>
      <form class="flex flex-col gap-4" @submit="onSubmit">
        <Field v-slot="{ field, errorMessage }" name="email">
          <AuthField
            label="Email"
            type="email"
            autocomplete="email"
            v-bind="field"
            :error="errorMessage"
          />
        </Field>
        <Field v-slot="{ field, errorMessage }" name="code">
          <AuthField
            label="Code"
            type="text"
            inputmode="numeric"
            v-bind="field"
            :error="errorMessage"
          />
        </Field>
        <Field v-slot="{ field, errorMessage }" name="new_password">
          <AuthField
            label="New password"
            type="password"
            autocomplete="new-password"
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
          Reset password
        </button>
      </form>
    </div>
  </main>
</template>
