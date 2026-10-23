<script setup lang="ts">
import { ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess, type IssuedSession, type TenantChoice } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { loginSchema } from '../schemas';

interface LoginFormProps {
  initialEmail?: string;
}

const props = withDefaults(defineProps<LoginFormProps>(), {
  initialEmail: '',
});

const emit = defineEmits<{ success: [session: IssuedSession] }>();

const client = useAuthClient();
const formError = ref<string>('');
const tenants = ref<TenantChoice[]>([]);
const needsTenant = ref<boolean>(false);

const form = useForm({
  validationSchema: toZod4TypedSchema(loginSchema),
  initialValues: { email: props.initialEmail, password: '', tenant_id: '' },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await client.request<IssuedSession>('auth/login', {
    method: 'POST',
    body: {
      email: values.email,
      password: values.password,
      ...(values.tenant_id ? { tenant_id: values.tenant_id } : {}),
    },
  });

  if (isSuccess(result.body)) {
    emit('success', result.body.data);
    return;
  }
  if (result.status === 409 && result.body.data?.tenants) {
    tenants.value = result.body.data.tenants;
    needsTenant.value = true;
  }
  formError.value = result.body.message;
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
    <Field v-slot="{ field, errorMessage }" name="password">
      <AuthField
        label="Password"
        type="password"
        autocomplete="current-password"
        v-bind="field"
        :error="errorMessage"
      />
    </Field>
    <Field v-if="needsTenant" v-slot="{ field, errorMessage }" name="tenant_id">
      <label class="flex flex-col gap-2">
        <span class="text-ink-secondary text-sm font-medium">Business</span>
        <select
          v-bind="field"
          class="border-border-soft bg-surface-canvas text-ink-primary rounded-full border px-4 py-3"
        >
          <option value="">Choose a business</option>
          <option v-for="tenant in tenants" :key="tenant.id" :value="tenant.id">
            {{ tenant.name }}
          </option>
        </select>
        <span v-if="errorMessage" class="text-danger text-sm">{{
          errorMessage
        }}</span>
      </label>
    </Field>
    <p v-if="formError" class="text-danger text-sm" role="alert">
      {{ formError }}
    </p>
    <button
      type="submit"
      class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
      :disabled="form.isSubmitting.value"
    >
      {{ form.isSubmitting.value ? 'Signing in...' : 'Log in' }}
    </button>
  </form>
</template>
