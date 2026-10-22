<script setup lang="ts">
import { ref, watch, type HTMLAttributes } from 'vue';
import { cn } from '../utils';

interface PriceInputProps {
  isMin?: boolean;
  otherValue?: number;
  class?: HTMLAttributes['class'];
}

const props = defineProps<PriceInputProps>();
const value = defineModel<number>({ required: true });

const inputRef = ref<HTMLInputElement | null>(null);
const isFocused = ref<boolean>(false);
const displayValue = ref<string>('');

function getScale(approxThousands: number) {
  return approxThousands >= 1000 ? 1_000_000 : 1_000;
}

function syncDisplayValue() {
  if (isFocused.value) {
    displayValue.value = value.value.toLocaleString('en-US', {
      minimumFractionDigits: value.value % 1 !== 0 ? 1 : 0,
      maximumFractionDigits: 2,
    });
  } else {
    const scale = getScale(value.value / 1000);
    const scaled = value.value / scale;
    const unit = scale === 1_000_000 ? 'M' : 'K';
    const formatted = scaled.toLocaleString('en-US', {
      minimumFractionDigits: scaled % 1 !== 0 ? 1 : 0,
      maximumFractionDigits: 2,
    });
    displayValue.value = `${formatted}${unit}`;
  }
}

watch([value, isFocused], syncDisplayValue, { immediate: true });

function commitValue(currentDisplay: string = displayValue.value) {
  const numStr = currentDisplay.replace(/[^0-9.]/g, '');
  let num = Number(numStr);
  if (isNaN(num) || num < 0) return;

  if (props.otherValue !== undefined) {
    if (props.isMin && num > props.otherValue) {
      num = props.otherValue;
    } else if (!props.isMin && num < props.otherValue) {
      num = props.otherValue;
    }
  }

  value.value = num;
}

function handleChange(e: Event) {
  const target = e.target as HTMLInputElement;
  const oldValue = displayValue.value;
  const selectionStart = target.selectionStart;

  let newRaw = target.value.replace(/[^0-9.]/g, '');

  const parts = newRaw.split('.');
  if (parts.length > 2) {
    newRaw = parts[0] + '.' + parts.slice(1).join('');
  }

  const [integer, decimal] = newRaw.split('.');
  const formattedInteger = integer.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  const newDisplay = decimal
    ? `${formattedInteger}.${decimal}`
    : formattedInteger;

  displayValue.value = newDisplay;

  const lengthDiff = newDisplay.length - oldValue.length;
  const newCursorPos = (selectionStart ?? 0) + lengthDiff;

  requestAnimationFrame(() => {
    inputRef.value?.setSelectionRange(newCursorPos, newCursorPos);
  });

  commitValue(newDisplay);
}

function handleBlur() {
  isFocused.value = false;
  commitValue();
}

function handleKeyDown(e: KeyboardEvent) {
  if (
    !/[0-9.]/.test(e.key) &&
    !['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Tab'].includes(e.key)
  ) {
    e.preventDefault();
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    inputRef.value?.blur();
  }
}
</script>

<template>
  <div
    :class="
      cn(
        'border-input text-ink-primary relative flex h-12 w-full min-w-0 items-center rounded-xl border-b text-base transition-all duration-300 lg:border-0',
        props.class,
      )
    "
  >
    <span class="pointer-events-none">&#8358;</span>

    <input
      ref="inputRef"
      type="text"
      inputmode="decimal"
      :value="displayValue"
      :class="
        cn(
          'placeholder:text-ink-muted ml-1 h-full w-full bg-transparent outline-none focus:cursor-text',
        )
      "
      @input="handleChange"
      @keydown="handleKeyDown"
      @focus="isFocused = true"
      @blur="handleBlur"
    />
  </div>
</template>
