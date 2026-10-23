<script setup lang="ts">
import { computed, type HTMLAttributes } from 'vue';
import { Textarea } from '../../components/ui';
import { cn } from '../utils';

interface TextAreaInputProps {
  label?: string;
  required?: boolean;
  error?: string | boolean;
  maxLength?: number;
  showWordCount?: boolean;
  placeholder?: string;
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<TextAreaInputProps>(), {
  required: false,
  showWordCount: false,
});

const value = defineModel<string>({ default: '' });

const wordCount = computed(
  () => value.value.trim().split(/\s+/).filter(Boolean).length,
);

const hasError = computed(
  () => !!props.error && typeof props.error === 'string',
);

const finalPlaceholder = computed(
  () => props.placeholder ?? "Tell us what you'd love to see in your store...",
);
</script>

<template>
  <div :class="cn('flex flex-col gap-2', props.class)">
    <label
      v-if="label"
      :for="label"
      class="text-ink-primary flex items-center gap-1 text-base font-medium"
    >
      {{ label }}
      <span v-if="required" class="text-danger">*</span>
    </label>

    <div class="relative">
      <Textarea
        :id="label"
        v-model="value"
        :placeholder="finalPlaceholder"
        :maxlength="maxLength"
        :class="
          cn(
            'border-input text-ink-secondary caret-ink-primary min-h-30 w-full resize-y rounded-2xl border bg-white p-5 text-base shadow-xs',
            'placeholder:text-ink-muted transition-all duration-300 ease-in-out placeholder:text-base',
            'hover:border-border-soft hover:shadow-sm',
            'focus:border-border-soft focus:ring-ink-primary/30 focus:shadow-md focus:ring-1',
            hasError &&
              'border-danger focus:border-danger focus:ring-danger/30',
            'disabled:cursor-not-allowed disabled:opacity-60',
          )
        "
      />

      <div
        v-if="showWordCount && maxLength"
        :class="
          cn(
            'absolute right-4 bottom-3 text-xs transition-colors duration-200',
            wordCount > maxLength * 0.9 ? 'text-warning' : 'text-ink-muted',
          )
        "
      >
        {{ wordCount }}/{{ maxLength }} words
      </div>
    </div>

    <p v-if="hasError" class="text-danger text-sm">{{ error }}</p>
  </div>
</template>
