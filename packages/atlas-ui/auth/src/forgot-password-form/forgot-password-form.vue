<script setup lang="ts">
import { ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { forgotPasswordSchema } from '../schemas';

const emit = defineEmits<{ sent: [email: string] }>();

const client = useAuthClient();
const formError = ref<string>('');

const form = useForm({
  validationSchema: toZod4TypedSchema(forgotPasswordSchema),
  initialValues: { email: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await client.request<null>('auth/forgot-password', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  emit('sent', values.email);
});
</script>

<template>
  <form class="flex flex-col gap-4" novalidate @submit="onSubmit">
    <Field v-slot="{ field, errorMessage }" name="email">
      <AuthField
        label="Email"
        type="email"
        autocomplete="email"
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
      {{ form.isSubmitting.value ? 'Sending...' : 'Send code' }}
    </button>
  </form>
</template>
