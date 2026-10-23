import { onBeforeUnmount, onMounted, type Ref } from 'vue';
import { revealOnScroll } from './presets';

export function useScrollReveal(root: Ref<HTMLElement | null>): void {
  let stop: (() => void) | null = null;

  onMounted(() => {
    if (!root.value) {
      return;
    }
    stop = revealOnScroll(root.value);
  });

  onBeforeUnmount(() => {
    if (stop) {
      stop();
    }
  });
}
