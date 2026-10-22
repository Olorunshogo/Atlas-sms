<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  useId,
  watch,
  type HTMLAttributes,
} from 'vue';
import { ChevronDown, Search } from 'lucide-vue-next';
import { cn } from '../utils';
import { FormLabel } from '../form-label';
import type { SelectOption } from './types';

/**
 * Button-trigger + absolute-positioned dropdown, hand-rolled (no headless
 * library) - ported from the reference source's DashSelectInput. Click-outside and
 * keyboard-toggle are managed directly rather than via a portal, since the
 * dropdown only ever needs to sit relative to its own trigger.
 */

interface SelectInputProps {
  label: string;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  searchable?: boolean;
  error?: boolean;
  errorMessage?: string;
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<SelectInputProps>(), {
  placeholder: 'Select...',
  disabled: false,
  searchable: false,
  error: false,
});

const value = defineModel<string>();

const selectId = useId();
const rootRef = ref<HTMLDivElement>();
const searchRef = ref<HTMLInputElement>();

const open = ref<boolean>(false);
const query = ref<string>('');

const hasError = computed(() => props.error || !!props.errorMessage);

const selected = computed(() =>
  props.options.find((opt) => opt.value === value.value),
);

const visibleOptions = computed(() => {
  if (!props.searchable || query.value.trim() === '') return props.options;
  const q = query.value.toLowerCase();
  return props.options
    .filter(
      (opt) =>
        opt.label.toLowerCase().includes(q) ||
        opt.sublabel?.toLowerCase().includes(q),
    )
    .sort((a, b) => a.label.localeCompare(b.label));
});

function toggleOpen(): void {
  if (props.disabled) return;
  open.value = !open.value;
}

function handleSelect(option: SelectOption): void {
  value.value = option.value;
  open.value = false;
}

function handleClickOutside(e: MouseEvent): void {
  if (rootRef.value && !rootRef.value.contains(e.target as Node)) {
    open.value = false;
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleClickOutside);
});

watch(open, (isOpen) => {
  if (isOpen && props.searchable) {
    nextTick(() => searchRef.value?.focus());
  }
  if (!isOpen) query.value = '';
});
</script>

<template>
  <div
    ref="rootRef"
    :class="
      cn('font-outfit relative flex w-full flex-col gap-1.5', props.class)
    "
  >
    <FormLabel v-if="label" :html-for="selectId" class="block">{{
      label
    }}</FormLabel>

    <div
      :id="selectId"
      role="button"
      tabindex="0"
      :aria-expanded="open"
      :class="
        cn(
          'border-input bg-surface-canvas flex w-full cursor-pointer items-center justify-between gap-2 rounded-lg border px-4 py-3 whitespace-nowrap transition-colors duration-200 select-none',
          hasError && 'border-danger',
          disabled && 'cursor-not-allowed opacity-50',
        )
      "
      @click="toggleOpen"
      @keydown.enter="toggleOpen"
    >
      <span
        :class="
          cn(
            'text-body-md font-medium',
            selected ? 'text-ink-primary' : 'text-ink-secondary',
          )
        "
      >
        {{ selected?.label ?? placeholder }}
      </span>
      <ChevronDown
        :class="
          cn(
            'text-ink-secondary h-4 w-4 shrink-0 transition-transform duration-200',
            open && 'rotate-180',
          )
        "
      />
    </div>

    <Transition
      enter-active-class="transition-all duration-150 ease-out"
      enter-from-class="opacity-0 -translate-y-1.5 scale-y-95"
      enter-to-class="opacity-100 translate-y-0 scale-y-100"
      leave-active-class="transition-all duration-150 ease-out"
      leave-from-class="opacity-100 translate-y-0 scale-y-100"
      leave-to-class="opacity-0 -translate-y-1.5 scale-y-95"
    >
      <div
        v-if="open"
        class="bg-surface-canvas border-input absolute top-full left-0 z-50 mt-1 min-w-full origin-top rounded-lg border shadow-lg"
      >
        <div v-if="searchable" class="border-input border-b px-3 py-2">
          <div class="flex items-center gap-2">
            <Search class="text-ink-secondary h-3.5 w-3.5 shrink-0" />
            <input
              ref="searchRef"
              v-model="query"
              type="text"
              placeholder="Search..."
              class="text-body-sm text-ink-primary placeholder:text-ink-secondary/50 w-full bg-transparent outline-none"
            />
          </div>
        </div>

        <div class="max-h-90 overflow-y-auto">
          <p
            v-if="visibleOptions.length === 0"
            class="text-ink-secondary text-body-sm px-4 py-3"
          >
            No results.
          </p>
          <div
            v-for="opt in visibleOptions"
            :key="opt.value"
            role="option"
            :aria-selected="opt.value === value"
            :class="
              cn(
                'cursor-pointer px-4 py-2.5 transition-colors duration-150 first:rounded-t-lg last:rounded-b-lg',
                opt.value === value
                  ? 'bg-surface-sunken text-ink-primary'
                  : 'text-ink-secondary hover:bg-surface-sunken hover:text-ink-primary hover:font-semibold',
              )
            "
            @click="handleSelect(opt)"
          >
            <p
              :class="cn('text-body-sm', opt.value === value && 'font-medium')"
            >
              {{ opt.label }}
            </p>
            <p
              v-if="opt.sublabel"
              class="text-label-sm text-ink-secondary mt-0.5 truncate"
            >
              {{ opt.sublabel }}
            </p>
          </div>
        </div>
      </div>
    </Transition>

    <p v-if="errorMessage" class="text-danger text-xs">
      {{ errorMessage }}
    </p>
  </div>
</template>
