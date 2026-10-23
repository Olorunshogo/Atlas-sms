<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import { Field, useForm } from 'vee-validate';
import { isSuccess } from '@atlas/api';
import { toZod4TypedSchema } from '@atlas/validation';
import { AuthField } from '../auth-field';
import { useAuthClient } from '../client';
import { verifyEmailSchema } from '../schemas';

interface VerifyEmailFormProps {
  /** Known from signup or the session; when empty the form asks for it. */
  email?: string;
}

const props = withDefaults(defineProps<VerifyEmailFormProps>(), {
  email: '',
});

const emit = defineEmits<{ verified: [email: string] }>();

// Matches the server's one-code-a-minute limit
const RESEND_COOLDOWN_SECONDS = 60;

const client = useAuthClient();
const formError = ref<string>('');
const resendMessage = ref<string>('');
const resendError = ref<string>('');
const resending = ref<boolean>(false);
const cooldown = ref<number>(0);
let timer: ReturnType<typeof setInterval> | null = null;

const form = useForm({
  validationSchema: toZod4TypedSchema(verifyEmailSchema),
  initialValues: { email: props.email, code: '' },
});

const resendLabel = computed((): string => {
  if (resending.value) {
    return 'Sending...';
  }
  return cooldown.value > 0
    ? `Send a new code in ${cooldown.value}s`
    : 'Send a new code';
});

function startCooldown(): void {
  cooldown.value = RESEND_COOLDOWN_SECONDS;
  stopTimer();
  timer = setInterval(() => {
    cooldown.value -= 1;
    if (cooldown.value <= 0) {
      stopTimer();
    }
  }, 1000);
}

function stopTimer(): void {
  if (timer) {
    clearInterval(timer);
    timer = null;
  }
}

onBeforeUnmount(stopTimer);

const onSubmit = form.handleSubmit(async (values) => {
  formError.value = '';
  const result = await client.request<null>('auth/verify-email', {
    method: 'POST',
    body: values,
  });
  if (!isSuccess(result.body)) {
    formError.value = result.body.message;
    return;
  }
  emit('verified', values.email);
});

async function resend(): Promise<void> {
  const { valid } = await form.validateField('email');
  if (!valid) {
    return;
  }
  resendMessage.value = '';
  resendError.value = '';
  resending.value = true;
  const result = await client.request<null>('auth/resend-verification', {
    method: 'POST',
    body: { email: form.values.email },
  });
  resending.value = false;
  if (isSuccess(result.body)) {
    resendMessage.value = result.body.message;
    startCooldown();
    return;
  }
  resendError.value = result.body.message;
  if (result.status === 429) {
    startCooldown();
  }
}
</script>

<template>
  <form class="flex flex-col gap-4" novalidate @submit="onSubmit">
    <Field v-if="!props.email" v-slot="{ field, errorMessage }" name="email">
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
    <p v-if="formError" class="text-danger text-sm" role="alert">
      {{ formError }}
    </p>
    <button
      type="submit"
      class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
      :disabled="form.isSubmitting.value"
    >
      {{ form.isSubmitting.value ? 'Verifying...' : 'Verify' }}
    </button>
    <button
      type="button"
      data-test="resend"
      class="text-ink-secondary w-fit text-sm underline disabled:no-underline disabled:opacity-60"
      :disabled="resending || cooldown > 0"
      @click="resend"
    >
      {{ resendLabel }}
    </button>
    <p v-if="resendMessage" class="text-success text-sm">
      {{ resendMessage }}
    </p>
    <p v-if="resendError" class="text-danger text-sm" role="alert">
      {{ resendError }}
    </p>
  </form>
</template>
