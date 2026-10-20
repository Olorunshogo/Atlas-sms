<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../utils/api';

const route = useRoute();
const formError = ref<string>('');
const email = computed((): string => {
  return typeof route.query.email === 'string' ? route.query.email : '';
});

const schema = z.object({
  code: z.string().min(6, 'Enter the 6-digit code').max(6, 'Enter the 6-digit code'),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
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
  await navigateTo({ path: '/login', query: { email: email.value } });
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Verify email</h1>
    <p class="text-ink-secondary">Enter the code sent to {{ email || 'your email' }}. Then log in with the password you just set.</p>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="code">
        <AuthField label="Code" type="text" inputmode="numeric" autocomplete="one-time-code" v-bind="field" :error="errorMessage" />
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        Verify
      </button>
    </form>
  </main>
</template>
