import { computed, ref, type ComputedRef, type Ref } from 'vue';

/*
 * One lifecycle for every dialog that submits.
 * Success stays available so a panel can show it.
 * Callers that should leave immediately close from status === 'success'.
 */

export type DialogSubmissionStatus =
  | 'idle'
  | 'submitting'
  | 'error'
  | 'success';

export interface UseDialogSubmissionResult<T> {
  status: Ref<DialogSubmissionStatus>;
  error: Ref<string | null>;
  fieldErrors: Ref<Record<string, string>>;
  data: Ref<T | null>;
  isSubmitting: ComputedRef<boolean>;
  run: (action: () => Promise<T>) => Promise<T | undefined>;
  reset: () => void;
}

export function useDialogSubmission<T = void>(): UseDialogSubmissionResult<T> {
  const status = ref<DialogSubmissionStatus>('idle');
  const error = ref<string | null>(null);
  const fieldErrors = ref<Record<string, string>>({});
  const data = ref<T | null>(null) as Ref<T | null>;

  const isSubmitting = computed((): boolean => {
    return status.value === 'submitting';
  });

  function reset(): void {
    status.value = 'idle';
    error.value = null;
    fieldErrors.value = {};
    data.value = null;
  }

  async function run(action: () => Promise<T>): Promise<T | undefined> {
    status.value = 'submitting';
    error.value = null;
    fieldErrors.value = {};

    try {
      const result = await action();
      data.value = result;
      status.value = 'success';
      return result;
    } catch (caught) {
      error.value =
        caught instanceof Error
          ? caught.message
          : 'Something went wrong. Please try again.';
      status.value = 'error';
      return undefined;
    }
  }

  return {
    status,
    error,
    fieldErrors,
    data,
    isSubmitting,
    run,
    reset,
  };
}
