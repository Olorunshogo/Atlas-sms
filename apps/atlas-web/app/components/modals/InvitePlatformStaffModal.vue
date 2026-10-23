<script setup lang="ts">
import { Modal } from '@atlas/ui';
import { apiRequest, isSuccess } from '../../utils/api';

/**
 * The reference modal: `v-model` opens it, `data-gsap-field` marks what staggers in,
 * `blocked` holds it open while the request runs, and the form resets on `after-leave`.
 */
const open = defineModel<boolean>('open', { default: false });

const emit = defineEmits<{ invited: [message: string] }>();

const email = ref<string>('');
const error = ref<string>('');
const sending = ref<boolean>(false);

async function send(close: () => void): Promise<void> {
  if (sending.value) {
    return;
  }
  error.value = '';
  sending.value = true;
  const result = await apiRequest<null>('v1/auth/platform/invite', {
    method: 'POST',
    body: { email: email.value },
  });
  sending.value = false;
  if (!isSuccess(result.body)) {
    error.value = result.body.message;
    return;
  }
  emit('invited', result.body.message);
  close();
}

function resetState(): void {
  email.value = '';
  error.value = '';
}
</script>

<template>
  <Modal
    v-model:open="open"
    title="Invite platform staff"
    description="Only platform admins can add another. A new email gets an invite link; an existing account gets the grant immediately."
    :blocked="sending"
    @after-leave="resetState"
  >
    <template #default="{ close }">
      <form class="flex flex-col gap-4" @submit.prevent="send(close)">
        <label data-gsap-field class="flex flex-col gap-2">
          <span class="text-ink-secondary text-sm font-medium">Email</span>
          <input
            v-model="email"
            type="email"
            required
            autocomplete="email"
            :disabled="sending"
            class="border-border-soft bg-surface-canvas rounded-full border px-4 py-3"
          />
        </label>
        <p
          v-if="error"
          data-gsap-field
          class="text-danger text-sm"
          role="alert"
        >
          {{ error }}
        </p>
        <div data-gsap-field class="flex items-center justify-end gap-3">
          <button
            type="button"
            class="text-ink-secondary px-4 py-3 text-sm font-semibold"
            :disabled="sending"
            @click="close"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="bg-brand text-ink-inverse rounded-full px-4 py-3 text-sm font-semibold"
            :disabled="sending"
          >
            {{ sending ? 'Sending...' : 'Send invite' }}
          </button>
        </div>
      </form>
    </template>
  </Modal>
</template>
