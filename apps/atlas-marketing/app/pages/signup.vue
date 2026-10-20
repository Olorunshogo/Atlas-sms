<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../utils/api';

const formError = ref<string>('');

const schema = z.object({
  full_name: z.string().min(1, 'Name is required'),
  tenant_name: z.string().min(1, 'Business name is required'),
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
  phone: z.string().min(7, 'Enter a phone number'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
  initialValues: { full_name: '', tenant_name: '', email: '', phone: '', password: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await apiRequest<{ email: string }>('v1/auth/signup', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  await navigateTo({ path: '/verify-email', query: { email: result.body.data.email } });
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Create your business</h1>
    <p class="text-ink-secondary">Signup opens a new tenant and makes you the owner. You will verify email before you can log in.</p>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="full_name">
        <AuthField label="Your name" type="text" autocomplete="name" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="tenant_name">
        <AuthField label="Business name" type="text" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="email">
        <AuthField label="Email" type="email" autocomplete="email" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="phone">
        <AuthField label="Phone" type="tel" autocomplete="tel" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="password">
        <AuthField label="Password" type="password" autocomplete="new-password" v-bind="field" :error="errorMessage" />
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        {{ form.isSubmitting.value ? 'Creating...' : 'Create tenant' }}
      </button>
    </form>
  </main>
</template>
