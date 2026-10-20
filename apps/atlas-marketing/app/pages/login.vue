<script setup lang="ts">
import { toTypedSchema } from '@vee-validate/zod';
import { useForm, Field } from 'vee-validate';
import { z } from 'zod';
import { apiRequest, isSuccess, type TenantChoice } from '../utils/api';

const route = useRoute();
const formError = ref<string>('');
const tenants = ref<TenantChoice[]>([]);
const needsTenant = ref<boolean>(false);

const schema = z.object({
  email: z.string().min(1, 'Email is required').email('Enter a valid email'),
  password: z.string().min(8, 'Password must be at least 8 characters'),
  tenant_id: z.string().optional(),
});

const form = useForm({
  validationSchema: toTypedSchema(schema),
  initialValues: {
    email: typeof route.query.email === 'string' ? route.query.email : '',
    password: '',
    tenant_id: '',
  },
});

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await apiRequest<unknown>('v1/auth/login', {
    method: 'POST',
    body: {
      email: values.email,
      password: values.password,
      ...(values.tenant_id ? { tenant_id: values.tenant_id } : {}),
    },
  });

  if (result.status === 409 && !isSuccess(result.body) && result.body.data?.tenants) {
    tenants.value = result.body.data.tenants;
    needsTenant.value = true;
    formError.value = result.body.message;
    return;
  }

  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }

  await navigateTo('/dashboard');
});
</script>

<template>
  <main class="mx-auto flex w-full max-w-md flex-col gap-6 px-4 py-16">
    <h1 class="text-3xl font-medium">Log in</h1>
    <form class="flex flex-col gap-4" @submit="onSubmit">
      <Field v-slot="{ field, errorMessage }" name="email">
        <AuthField label="Email" type="email" autocomplete="email" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-slot="{ field, errorMessage }" name="password">
        <AuthField label="Password" type="password" autocomplete="current-password" v-bind="field" :error="errorMessage" />
      </Field>
      <Field v-if="needsTenant" v-slot="{ field, errorMessage }" name="tenant_id">
        <label class="flex flex-col gap-2">
          <span class="text-sm font-medium text-ink-secondary">Tenant</span>
          <select
            v-bind="field"
            class="rounded-full border border-border-soft bg-surface-canvas px-4 py-3 text-ink-primary"
          >
            <option value="">Choose a tenant</option>
            <option v-for="tenant in tenants" :key="tenant.id" :value="tenant.id">{{ tenant.name }}</option>
          </select>
          <span v-if="errorMessage" class="text-sm text-danger">{{ errorMessage }}</span>
        </label>
      </Field>
      <p v-if="formError" class="text-sm text-danger">{{ formError }}</p>
      <button type="submit" class="rounded-full bg-brand px-4 py-3 text-sm font-semibold text-ink-inverse" :disabled="form.isSubmitting.value">
        {{ form.isSubmitting.value ? 'Signing in...' : 'Log in' }}
      </button>
    </form>
    <div class="flex flex-col gap-2 text-sm text-ink-secondary">
      <NuxtLink to="/forgot-password" class="w-fit underline">Forgot password</NuxtLink>
      <NuxtLink to="/signup" class="w-fit underline">Create a tenant</NuxtLink>
    </div>
  </main>
</template>
