<script setup lang="ts">
import { toZod4TypedSchema } from '@atlas/validation';
import { Field, useForm } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess } from '../../utils/api';
import { usePageSeo } from '../../composables/use-page-seo';

usePageSeo({
  title: 'Forgot password',
  description: 'Request a password reset for an AtlasSMS account.',
  path: '/auth/forgot-password',
  noIndex: true,
});

const formError = ref<string>('');
const sentEmail = ref<string>('');

const schema = z.object({
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
});

const form = useForm({
  validationSchema: toZod4TypedSchema(schema),
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
  await navigateTo({
    path: '/auth/reset-password',
    query: { email: values.email },
  });
});
</script>

<template>
  <main
    class="layout-max-width px-section-px py-section-py sm:px-section-px-sm lg:px-section-px-lg w-full"
  >
    <div class="mx-auto flex w-full max-w-112 flex-col gap-6">
      <h1 class="text-3xl font-medium">Forgot password</h1>
      <p class="text-ink-secondary">
        If the email has an account, we send a reset code. The response is the
        same either way.
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
        <p v-if="formError" class="text-danger text-sm">{{ formError }}</p>
        <p v-if="sentEmail" class="text-ink-secondary text-sm">
          Check {{ sentEmail }}.
        </p>
        <button
          type="submit"
          class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
          :disabled="form.isSubmitting.value"
        >
          Send code
        </button>
      </form>
    </div>
  </main>
</template>
