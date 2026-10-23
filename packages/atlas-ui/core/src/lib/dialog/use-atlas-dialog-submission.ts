import { ref, type Ref } from 'vue';

export type DialogSubmissionStatus =
  | 'idle'
  | 'submitting'
  | 'error'
  | 'success';

export interface UseAtlasDialogSubmissionResult {
  status: Ref<DialogSubmissionStatus>;
  error: Ref<string>;
  run: (fn: () => Promise<void>) => Promise<void>;
  reset: () => void;
}

/**
 * Same contract as the reference source's useDialogSubmission - wires a submit action to
 * the loading/error/success trio (SubmitButton, DialogErrorBanner,
 * DialogSuccessPanel) without every consumer re-deriving this state.
 */
export function useAtlasDialogSubmission(): UseAtlasDialogSubmissionResult {
  const status = ref<DialogSubmissionStatus>('idle');
  const error = ref<string>('');

  async function run(fn: () => Promise<void>): Promise<void> {
    if (status.value === 'submitting') return;
    status.value = 'submitting';
    error.value = '';
    try {
      await fn();
      status.value = 'success';
    } catch (err) {
      status.value = 'error';
      error.value =
        err instanceof Error
          ? err.message
          : 'Something went wrong. Please try again.';
    }
  }

  function reset(): void {
    status.value = 'idle';
    error.value = '';
  }

  return { status, error, run, reset };
}
