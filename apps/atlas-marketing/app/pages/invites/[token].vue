<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../../utils/api';

const route = useRoute();
const formError = ref<string>('');
const token = computed((): string => {
  return typeof route.params.token === 'string' ? route.params.token : '';
});

const schema = z.object({
  full_name: z.string().min(1, 'Name is required'),
  phone: z.string().min(7, 'Enter a phone number'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
  initialValues: { full_name: '', phone: '', password: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await apiRequest<unknown>(`v1/auth/invites/${token.value}/accept`, {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  await navigateTo('/login');
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Accept invite</h1>
    <p class="text-ink-secondary">Set a password for this invite. You will log in next. Accepting does not start a session.</p>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="full_name">
        <AuthField label="Your name" type="text" autocomplete="name" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="phone">
        <AuthField label="Phone" type="tel" autocomplete="tel" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="password">
        <AuthField label="Password" type="password" autocomplete="new-password" v-bind="field" :error="errorMessage" />
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        Accept and continue to login
      </button>
    </form>
  </main>
</template>
