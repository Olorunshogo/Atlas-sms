<script setup lang="ts">
import { ref, type HTMLAttributes } from 'vue';
import { ChevronDown } from 'lucide-vue-next';
import { cn } from '../utils';
import type { FaqItem } from './types';

/**
 * Fully array-driven: pass `items` and it renders the same accordion
 * regardless of app/page - landing-page marketing FAQs, an org's KYC-help
 * section, etc. all use this one component instead of hand-rolling their own.
 */

interface SectionFaqProps {
  items: FaqItem[];
  title?: string;
  class?: HTMLAttributes['class'];
}

defineProps<SectionFaqProps>();

const openIndex = ref<number | null>(null);

function toggle(index: number): void {
  openIndex.value = openIndex.value === index ? null : index;
}
</script>

<template>
  <section
    :class="cn('font-outfit grid w-full grid-cols-1 gap-6', $props.class)"
  >
    <h2 v-if="title" class="text-ink-primary text-2xl font-bold md:text-3xl">
      {{ title }}
    </h2>

    <div class="grid grid-cols-1 gap-3">
      <div
        v-for="(item, i) in items"
        :key="item.question"
        :class="
          cn(
            'border-border-soft bg-surface-canvas cursor-pointer rounded-xl border transition-all duration-300 ease-in-out',
            openIndex === i
              ? 'border-brand/60 shadow-sm'
              : 'hover:border-border-firm',
          )
        "
        @click="toggle(i)"
      >
        <div class="flex items-start gap-4 p-5">
          <span
            class="bg-brand-soft text-brand-ink flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-xs font-bold"
          >
            {{ i + 1 }}
          </span>

          <h3
            :class="
              cn(
                'text-ink-primary flex-1 text-sm font-semibold md:text-base',
                openIndex === i && 'text-brand-ink',
              )
            "
          >
            {{ item.question }}
          </h3>

          <ChevronDown
            :class="
              cn(
                'text-ink-secondary shrink-0 transition-transform duration-300 ease-in-out',
                openIndex === i && 'text-brand-ink rotate-180',
              )
            "
          />
        </div>

        <Transition
          enter-active-class="transition-all duration-250 ease-in-out overflow-hidden"
          enter-from-class="max-h-0 opacity-0"
          enter-to-class="max-h-80 opacity-100"
          leave-active-class="transition-all duration-250 ease-in-out overflow-hidden"
          leave-from-class="max-h-80 opacity-100"
          leave-to-class="max-h-0 opacity-0"
        >
          <p
            v-if="openIndex === i"
            class="text-ink-secondary border-border-soft border-t px-5 pt-4 pb-5 pl-16 text-sm leading-relaxed"
          >
            {{ item.answer }}
          </p>
        </Transition>
      </div>
    </div>
  </section>
</template>
