<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { Search } from 'lucide-vue-next';
import { cn } from '../utils';

interface SearchFieldProps {
  placeholder?: string;
  label?: string;
  tone?: 'default' | 'inverse';
  class?: HTMLAttributes['class'];
}

const props = withDefaults(defineProps<SearchFieldProps>(), {
  placeholder: 'Search',
  tone: 'default',
});

const modelValue = defineModel<string>({ default: '' });
</script>

<template>
  <label class="flex flex-col gap-2">
    <span
      v-if="label"
      class="text-sm font-semibold"
      :class="tone === 'inverse' ? 'text-ink-inverse' : 'text-ink-secondary'"
      >{{ label }}</span
    >
    <span
      :class="
        cn(
          'rounded-control flex items-center gap-2 border px-3 py-3',
          tone === 'inverse'
            ? 'border-ink-inverse/25 text-ink-inverse'
            : 'border-border-soft bg-surface-raised text-ink-primary',
          props.class,
        )
      "
    >
      <Search class="size-4 shrink-0" aria-hidden="true" />
      <input
        v-model="modelValue"
        type="search"
        :placeholder="placeholder"
        class="w-full bg-transparent text-sm outline-none"
        :class="
          tone === 'inverse'
            ? 'placeholder:text-ink-inverse/60'
            : 'placeholder:text-ink-muted'
        "
      />
    </span>
  </label>
</template>
