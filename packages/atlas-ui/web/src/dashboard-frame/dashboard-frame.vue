<script setup lang="ts">
import type { Component } from 'vue';
import {
  ArrowLeftRight,
  Banknote,
  ClipboardList,
  Package,
  PackagePlus,
  Receipt,
  SlidersHorizontal,
  Store,
} from 'lucide-vue-next';
import { QuickAction, StatCard, StoreChip } from '@atlas/ui-core';

interface DashboardStat {
  label: string;
  value: string;
  sub: string;
  icon: Component;
}

interface DashboardAction {
  label: string;
  icon: Component;
}

interface DashboardFrameProps {
  storeName?: string;
  greeting?: string;
  summary?: string;
}

withDefaults(defineProps<DashboardFrameProps>(), {
  storeName: 'Main store',
  greeting: 'Good morning',
  summary: "Here's what needs your attention today.",
});

const stats: DashboardStat[] = [
  {
    label: "Today's sales",
    value: '\u2014',
    sub: 'Awaiting live data',
    icon: Banknote,
  },
  {
    label: 'Orders',
    value: '\u2014',
    sub: 'Awaiting live data',
    icon: ClipboardList,
  },
  {
    label: 'Low stock',
    value: '\u2014',
    sub: 'Items needing review',
    icon: Package,
  },
  {
    label: 'Stores',
    value: '\u2014',
    sub: 'Accessible locations',
    icon: Store,
  },
];

const actions: DashboardAction[] = [
  { label: 'Add product', icon: PackagePlus },
  { label: 'Stock transfer', icon: ArrowLeftRight },
  { label: 'View sales', icon: Receipt },
  { label: 'Stock adjustment', icon: SlidersHorizontal },
];
</script>

<template>
  <section
    class="border-border-soft bg-surface-raised rounded-card flex flex-col gap-6 border p-6 shadow-sm"
  >
    <header class="flex items-center justify-between gap-4">
      <div class="flex flex-col gap-1">
        <slot name="logo" />
        <p class="text-ink-muted text-sm">Store management</p>
      </div>
      <StoreChip :label="storeName" />
    </header>

    <div class="flex flex-col gap-1">
      <h2 class="text-ink-primary text-2xl font-semibold">{{ greeting }}</h2>
      <p class="text-ink-secondary">{{ summary }}</p>
    </div>

    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
      <StatCard
        v-for="stat in stats"
        :key="stat.label"
        :label="stat.label"
        :value="stat.value"
        :sub="stat.sub"
        :icon="stat.icon"
      />
    </div>

    <div class="flex flex-col gap-3">
      <h3 class="text-ink-primary font-semibold">Quick actions</h3>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <QuickAction
          v-for="action in actions"
          :key="action.label"
          :label="action.label"
          :icon="action.icon"
        />
      </div>
    </div>
  </section>
</template>
