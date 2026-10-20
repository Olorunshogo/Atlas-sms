<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../utils/api';

const formError = ref<string>('');
const sentEmail = ref<string>('');

const schema = z.object({
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
  initialValues: { email: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await apiRequest<unknown>('v1/auth/forgot-password', {
    method: 'POST',
    body: { email: values.email },
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  sentEmail.value = values.email;
  await navigateTo({ path: '/reset-password', query: { email: values.email } });
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Forgot password</h1>
    <p class="text-ink-secondary">If the email has an account, we send a reset code. The response is the same either way.</p>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="email">
        <AuthField label="Email" type="email" autocomplete="email" v-bind="field" :error="errorMessage" />
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <p v-if="sentEmail" class="text-sm text-ink-secondary">Check {{ sentEmail }}.</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        Send code
      </button>
    </form>
  </main>
</template>
