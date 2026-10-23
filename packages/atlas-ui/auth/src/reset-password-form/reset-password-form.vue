<script setup lang="ts">
import { ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { resetPasswordSchema } from '../schemas';

interface ResetPasswordFormProps {
  initialEmail?: string;
}

const props = withDefaults(defineProps<ResetPasswordFormProps>(), {
  initialEmail: '',
});

const emit = defineEmits<{ reset: [email: string] }>();

const client = useAuthClient();
const formError = ref<string>('');

const form = useForm({
  validationSchema: toZod4TypedSchema(resetPasswordSchema),
  initialValues: { email: props.initialEmail, code: '', new_password: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await client.request<null>('auth/reset-password', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  emit('reset', values.email);
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
    <Field v-slot="{ field, errorMessage }" name="code">
      <AuthField
        label="Code"
        type="text"
        inputmode="numeric"
        autocomplete="one-time-code"
        maxlength="6"
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
    <p v-if="formError" class="text-danger text-sm" role="alert">
      {{ formError }}
    </p>
    <button
      type="submit"
      class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
      :disabled="form.isSubmitting.value"
    >
      {{ form.isSubmitting.value ? 'Resetting...' : 'Reset password' }}
    </button>
  </form>
</template>
