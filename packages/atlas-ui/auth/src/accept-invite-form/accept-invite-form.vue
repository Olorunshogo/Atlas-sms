<script setup lang="ts">
import { ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { acceptInviteSchema } from '../schemas';

interface AcceptInviteFormProps {
  token: string;
}

const props = defineProps<AcceptInviteFormProps>();

const emit = defineEmits<{ accepted: [] }>();

const client = useAuthClient();
const formError = ref<string>('');

const form = useForm({
  validationSchema: toZod4TypedSchema(acceptInviteSchema),
  initialValues: { full_name: '', phone: '', password: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  if (!props.token) {
    formError.value =
      'This invite link is incomplete. Open it from the email again.';
    return;
  }
  const result = await client.request<null>(
    `auth/invites/${encodeURIComponent(props.token)}/accept`,
    { method: 'POST', body: values },
  );
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  emit('accepted');
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
      {{
        form.isSubmitting.value
          ? 'Accepting...'
          : 'Accept and continue to login'
      }}
    </button>
  </form>
</template>
