<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { MenuIcon } from 'lucide-vue-next';
import { Button } from '../../components/ui';
import { cn } from '../utils';

const props = defineProps<{ class?: HTMLAttributes['class'] }>();
</script>

<template>
  <Button
    variant="ghost"
    size="icon"
    :class="
      cn(
        'hover:bg-surface-raised relative flex w-fit cursor-pointer items-center justify-center rounded-md p-2 transition-all duration-300 ease-in-out hover:shadow-md lg:hidden',
        props.class,
      )
    "
  >
    <MenuIcon class="text-ink-primary h-6 w-6" />
    <span class="sr-only">Toggle menu</span>
  </Button>
</template>
