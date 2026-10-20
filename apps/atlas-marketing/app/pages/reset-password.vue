<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../utils/api';

const route = useRoute();
const formError = ref<string>('');

const schema = z.object({
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
  code: z.string().min(6, 'Enter the 6-digit code').max(6, 'Enter the 6-digit code'),
  new_password: z.string().min(8, 'Password must be at least 8 characters'),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
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
  await navigateTo({ path: '/login', query: { email: values.email } });
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Reset password</h1>
    <p class="text-ink-secondary">This signs you out of every device. Log in again with the new password.</p>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="email">
        <AuthField label="Email" type="email" autocomplete="email" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="code">
        <AuthField label="Code" type="text" inputmode="numeric" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="new_password">
        <AuthField label="New password" type="password" autocomplete="new-password" v-bind="field" :error="errorMessage" />
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        Reset password
      </button>
    </form>
  </main>
</template>
