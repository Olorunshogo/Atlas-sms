<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  ref,
  watch,
  type HTMLAttributes,
} from 'vue';
import { Input } from '../../components/ui';
import { Eye, EyeOff, Mail, User, ChevronDown } from 'lucide-vue-next';
import { cn } from '../utils';
import type { DynamicInputOption, PhoneCountry } from './types';

interface DynamicInputProps {
  label?: string;
  required?: boolean;
  error?: boolean | string;
  fieldType?:
    | 'text'
    | 'email'
    | 'tel'
    | 'password'
    | 'confirm-password'
    | 'phone'
    | 'select';
  placeholder?: string;
  disabled?: boolean;
  class?: HTMLAttributes['class'];

  options?: DynamicInputOption[];
  countries?: PhoneCountry[];
  country?: PhoneCountry;
  phoneValue?: string;
}

const props = withDefaults(defineProps<DynamicInputProps>(), {
  required: false,
  fieldType: 'text',
  options: () => [],
  countries: () => [],
  phoneValue: '',
});

const emit = defineEmits<{
  countryChange: [country: PhoneCountry];
  phoneChange: [value: string];
}>();

const value = defineModel<string>();

const visible = ref<boolean>(false);
const isCountryOpen = ref<boolean>(false);
const isSelectOpen = ref<boolean>(false);
const focusedIndex = ref<number>(-1);

const selectRef = ref<HTMLDivElement | null>(null);
const listboxRef = ref<HTMLDivElement | null>(null);
const countryRef = ref<HTMLDivElement | null>(null);

const hasError = computed(
  () => !!props.error || typeof props.error === 'string',
);
const isDisabled = computed(() => !!props.disabled);

function onOutsideSelectClick(e: MouseEvent) {
  if (selectRef.value && !selectRef.value.contains(e.target as Node)) {
    isSelectOpen.value = false;
    focusedIndex.value = -1;
  }
}

function onOutsideCountryClick(e: MouseEvent) {
  if (countryRef.value && !countryRef.value.contains(e.target as Node)) {
    isCountryOpen.value = false;
  }
}

watch(isSelectOpen, (open) => {
  if (open) {
    document.addEventListener('mousedown', onOutsideSelectClick);
  } else {
    document.removeEventListener('mousedown', onOutsideSelectClick);
  }
});

watch(isCountryOpen, (open) => {
  if (open) {
    document.addEventListener('mousedown', onOutsideCountryClick);
  } else {
    document.removeEventListener('mousedown', onOutsideCountryClick);
  }
});

watch(focusedIndex, (idx) => {
  if (!isSelectOpen.value || idx < 0 || !listboxRef.value) return;
  const item = listboxRef.value.children[idx] as HTMLElement | undefined;
  item?.scrollIntoView({ block: 'nearest' });
});

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onOutsideSelectClick);
  document.removeEventListener('mousedown', onOutsideCountryClick);
});

const selectedCountry = computed(() => props.country ?? props.countries[0]);

function handleCountrySelect(country: PhoneCountry) {
  emit('countryChange', country);
  isCountryOpen.value = false;
}

function handlePhoneInput(e: Event) {
  emit('phoneChange', (e.target as HTMLInputElement).value);
}

const finalPlaceholderSelect = computed(
  () => props.placeholder || 'Select an option',
);
const selectedOption = computed(() =>
  props.options.find((o) => o.value === value.value),
);

function handleSelectChange(optionValue: string) {
  value.value = optionValue;
  isSelectOpen.value = false;
  focusedIndex.value = -1;
}

function openSelect() {
  if (isDisabled.value) return;
  const nextOpen = !isSelectOpen.value;
  isSelectOpen.value = nextOpen;
  if (nextOpen) {
    const idx = props.options.findIndex((o) => o.value === value.value);
    focusedIndex.value = idx >= 0 ? idx : 0;
  }
}

function handleSelectKeyDown(e: KeyboardEvent) {
  if (isDisabled.value) return;

  if (!isSelectOpen.value) {
    if (['Enter', ' ', 'ArrowDown', 'ArrowUp'].includes(e.key)) {
      e.preventDefault();
      isSelectOpen.value = true;
      const idx = props.options.findIndex((o) => o.value === value.value);
      focusedIndex.value = idx >= 0 ? idx : 0;
    }
    return;
  }

  if (e.key === 'Escape' || e.key === 'Tab') {
    isSelectOpen.value = false;
    focusedIndex.value = -1;
    return;
  }

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    for (let i = focusedIndex.value + 1; i < props.options.length; i++) {
      if (!props.options[i].disabled) {
        focusedIndex.value = i;
        break;
      }
    }
  }

  if (e.key === 'ArrowUp') {
    e.preventDefault();
    for (let i = focusedIndex.value - 1; i >= 0; i--) {
      if (!props.options[i].disabled) {
        focusedIndex.value = i;
        break;
      }
    }
  }

  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    const focused = props.options[focusedIndex.value];
    if (focused && !focused.disabled) {
      handleSelectChange(focused.value);
    }
  }
}

const inputType = computed(() => {
  if (
    props.fieldType === 'password' ||
    props.fieldType === 'confirm-password'
  ) {
    return visible.value ? 'text' : 'password';
  }
  return props.fieldType;
});

const defaultPlaceholders: Record<string, string> = {
  text: 'Enter text...',
  email: 'example@example.com',
  tel: '+234 (0) 123-4567',
  password: 'At least 8 characters',
  'confirm-password': 'Confirm your password',
  phone: '',
};

const finalPlaceholder = computed(
  () => props.placeholder || defaultPlaceholders[props.fieldType],
);

const LeadingIcon = computed(() => {
  if (props.fieldType === 'email') return Mail;
  if (props.fieldType === 'text') return User;
  return null;
});
</script>

<template>
  <!-- Phone variant -->
  <div
    v-if="fieldType === 'phone'"
    :class="cn('flex flex-col gap-2', props.class)"
  >
    <label
      v-if="label"
      :for="`${label}-phone`"
      class="text-ink-primary flex cursor-default items-center gap-1 text-base font-semibold transition-all duration-300 ease-in-out hover:cursor-pointer"
    >
      {{ label }}
      <span v-if="required" class="text-danger">*</span>
    </label>

    <div
      :class="
        cn(
          'bg-background z-10 flex h-14 w-full items-center overflow-hidden overflow-visible rounded-full border shadow-xs transition-all duration-300 ease-in-out',
          hasError ? 'border-danger' : 'border-input',
          'focus-within:border-border-soft',
        )
      "
    >
      <div
        ref="countryRef"
        class="border-input relative flex h-full shrink-0 items-center border-r"
      >
        <button
          type="button"
          aria-haspopup="listbox"
          :aria-expanded="isCountryOpen"
          class="flex h-full w-full cursor-pointer items-center justify-between gap-1.5 bg-transparent px-2"
          @click="isCountryOpen = !isCountryOpen"
        >
          <div class="flex flex-1 items-center gap-2">
            <img
              v-if="selectedCountry?.flagImage"
              :src="selectedCountry.flagImage"
              :alt="selectedCountry.label"
              class="h-4 w-5 shrink-0 rounded-sm object-cover"
            />
            <span v-else class="bg-input h-4 w-5 shrink-0 rounded-sm" />
            <span class="text-sm font-medium">{{
              selectedCountry?.code ?? '+?'
            }}</span>
          </div>
          <ChevronDown :size="13" class="text-ink-muted" />
        </button>
        <div
          v-if="isCountryOpen"
          role="listbox"
          aria-label="Country code"
          class="border-input bg-background absolute top-full left-0 z-10 mt-2 max-h-64 w-fit cursor-pointer overflow-auto rounded-2xl border p-1 shadow-lg"
        >
          <div
            v-for="c in countries"
            :key="c.value"
            role="option"
            :aria-selected="c.value === selectedCountry?.value"
            :class="
              cn(
                'flex cursor-pointer items-center gap-2 rounded-xl px-3 py-2 text-sm transition-colors',
                c.value === selectedCountry?.value
                  ? 'bg-input text-ink-primary'
                  : 'text-ink-secondary hover:bg-surface-raised',
              )
            "
            @click="handleCountrySelect(c)"
          >
            <img
              v-if="c.flagImage"
              :src="c.flagImage"
              :alt="c.label"
              class="h-4 w-5 shrink-0 rounded-sm object-cover"
            />
            <span v-else class="bg-input h-4 w-5 shrink-0 rounded-sm" />
            <span class="font-medium">{{ c.code }}</span>
            <span class="text-ink-secondary">- {{ c.label }}</span>
          </div>
        </div>
      </div>

      <div class="h-full flex-1">
        <input
          :id="label ? `${label}-phone` : undefined"
          type="tel"
          inputmode="numeric"
          :placeholder="
            placeholder ?? `${selectedCountry?.phoneLength ?? 10} digits`
          "
          :value="phoneValue"
          :aria-label="label ?? 'Phone number'"
          class="caret-ink-secondary placeholder:text-ink-muted text-ink-primary h-full w-full cursor-default rounded-r-full bg-transparent px-4 text-base transition-all duration-300 ease-in-out placeholder:text-base hover:cursor-pointer focus:cursor-text focus:outline-none"
          @input="handlePhoneInput"
        />
      </div>
    </div>

    <p v-if="typeof error === 'string' && error" class="text-danger text-sm">
      {{ error }}
    </p>
  </div>

  <!-- Select variant -->
  <div
    v-else-if="fieldType === 'select'"
    :class="cn('flex flex-col gap-2', props.class)"
  >
    <label
      v-if="label"
      class="text-ink-primary flex cursor-default items-center gap-1 text-base font-medium transition-all duration-300 ease-in-out hover:cursor-pointer focus:cursor-text lg:font-medium"
    >
      {{ label }}
      <span v-if="required" class="text-danger">*</span>
    </label>

    <div
      ref="selectRef"
      class="relative cursor-default transition-all duration-300 ease-in-out hover:cursor-pointer"
    >
      <div
        role="combobox"
        aria-haspopup="listbox"
        :aria-expanded="isSelectOpen"
        :aria-disabled="isDisabled"
        :tabindex="isDisabled ? -1 : 0"
        :class="
          cn(
            'flex h-14 w-full cursor-pointer items-center justify-between gap-2 rounded-full border bg-white pr-3 pl-3 text-base shadow-xs transition-all duration-300 ease-in-out outline-none',
            hasError ? 'border-danger' : 'border-input',
            'focus:border-border-soft',
            isDisabled && 'pointer-events-none opacity-50',
          )
        "
        @click="openSelect"
        @keydown="handleSelectKeyDown"
      >
        <span
          :class="
            cn(
              'flex-1 truncate',
              selectedOption ? 'text-ink-primary' : 'text-ink-muted',
            )
          "
        >
          {{ selectedOption ? selectedOption.label : finalPlaceholderSelect }}
        </span>
        <ChevronDown
          :size="16"
          :class="
            cn(
              'text-ink-muted shrink-0 transition-transform duration-200',
              isSelectOpen && 'rotate-180',
            )
          "
        />
      </div>

      <div
        v-if="isSelectOpen"
        role="listbox"
        :aria-label="label ?? 'Options'"
        class="border-input absolute top-full left-0 z-50 mt-1 max-h-75 w-full overflow-hidden rounded-md border bg-white shadow-md"
      >
        <div ref="listboxRef" class="max-h-65 overflow-auto">
          <div
            v-for="(option, index) in options"
            :key="option.value"
            role="option"
            :aria-selected="option.value === value"
            :aria-disabled="option.disabled"
            :class="
              cn(
                'border-border-soft cursor-pointer border-b px-4 py-3 text-left text-base transition-all duration-300 ease-in-out last:border-b-0',
                option.value === value
                  ? 'bg-surface-sunken text-ink-primary font-medium'
                  : 'text-ink-primary',
                index === focusedIndex && option.value !== value
                  ? 'bg-ink-primary text-ink-inverse'
                  : '',
                !option.disabled && option.value !== value
                  ? 'hover:bg-ink-primary hover:text-ink-inverse'
                  : '',
                option.disabled && 'cursor-not-allowed opacity-40',
              )
            "
            @click="!option.disabled && handleSelectChange(option.value)"
          >
            {{ option.label }}
          </div>
        </div>
      </div>
    </div>

    <p v-if="typeof error === 'string' && error" class="text-danger text-sm">
      {{ error }}
    </p>
  </div>

  <!-- Standard variants (text, email, tel, password, confirm-password) -->
  <div v-else :class="cn('flex flex-col gap-2', props.class)">
    <label
      v-if="label"
      :for="label"
      class="text-ink-primary flex cursor-default items-center gap-1 text-base font-medium transition-all duration-300 ease-in-out hover:cursor-pointer focus:cursor-text lg:font-medium"
    >
      {{ label }}
      <span v-if="required" class="text-danger">*</span>
    </label>

    <div
      class="relative cursor-default transition-all duration-300 ease-in-out hover:cursor-pointer focus:cursor-text"
    >
      <div
        v-if="LeadingIcon"
        class="pointer-events-none absolute inset-y-0 left-3 flex items-center"
      >
        <component :is="LeadingIcon" :size="20" class="text-ink-muted" />
      </div>

      <Input
        :id="label"
        v-model="value"
        :type="inputType"
        :aria-label="label || fieldType"
        :placeholder="finalPlaceholder"
        :disabled="disabled"
        :class="
          cn(
            'h-14 w-full cursor-default rounded-full border text-base shadow-xs ring-0 transition-all duration-300 ease-in-out outline-none hover:cursor-pointer focus:cursor-text',
            LeadingIcon ? 'pl-10' : 'pl-3',
            fieldType === 'password' || fieldType === 'confirm-password'
              ? 'pr-10'
              : 'pr-3',
            hasError ? 'border-danger' : 'border-input',
            'border-input caret-ink-secondary focus:border-border-soft text-ink-primary placeholder:text-base',
          )
        "
      />

      <button
        v-if="fieldType === 'password' || fieldType === 'confirm-password'"
        type="button"
        :aria-label="visible ? 'Hide password' : 'Show password'"
        class="hover:text-ink-secondary absolute inset-y-0 right-0 flex cursor-pointer items-center px-3 transition-all duration-300 ease-in-out"
        @click="visible = !visible"
      >
        <EyeOff v-if="visible" :size="18" />
        <Eye v-else :size="18" />
      </button>
    </div>

    <p v-if="typeof error === 'string' && error" class="text-danger text-sm">
      {{ error }}
    </p>
  </div>
</template>
