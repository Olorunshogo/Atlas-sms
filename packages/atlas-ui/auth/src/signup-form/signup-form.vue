<script setup lang="ts">
import { ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { signupSchema } from '../schemas';

const emit = defineEmits<{ success: [email: string] }>();

const client = useAuthClient();
const formError = ref<string>('');

const form = useForm({
  validationSchema: toZod4TypedSchema(signupSchema),
  initialValues: {
    full_name: '',
    tenant_name: '',
    email: '',
    phone: '',
    password: '',
  },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await client.request<{ email: string }>('auth/signup', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  emit('success', result.body.data.email);
});
</script>

<template>
  <form class="flex flex-col gap-4" novalidate @submit="onSubmit">
    <Field v-slot="{ field, errorMessage }" name="full_name">
      <AuthField
        label="Your name"
        type="text"
        autocomplete="name"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <Field v-slot="{ field, errorMessage }" name="tenant_name">
      <AuthField
        label="Business name"
        type="text"
        autocomplete="organization"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <Field v-slot="{ field, errorMessage }" name="email">
      <AuthField
        label="Email"
        type="email"
        autocomplete="email"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <Field v-slot="{ field, errorMessage }" name="phone">
      <AuthField
        label="Phone"
        type="tel"
        autocomplete="tel"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <Field v-slot="{ field, errorMessage }" name="password">
      <AuthField
        label="Password"
        type="password"
        autocomplete="new-password"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <p v-if="formError" class="text-danger text-sm" role="alert">
      {{ formError }}
    </p>
    <button
      type="submit"
      class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
      :disabled="form.isSubmitting.value"
    >
      {{ form.isSubmitting.value ? 'Creating...' : 'Create business' }}
    </button>
  </form>
</template>
