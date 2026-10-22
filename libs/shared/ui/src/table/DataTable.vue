<script setup lang="ts" generic="TRow extends object">
import { ref } from 'vue';
import type {
  BulkAction,
  RowAction,
  TableColumn,
  TableSort,
  TableStateSnapshot,
} from './table-types';
import { useOptionalDialog } from '../dialog/use-dialog';
import { useTableState } from './use-table-state';

const props = withDefaults(
  defineProps<{
    rows: readonly TRow[];
    columns: readonly TableColumn<TRow>[];
    caption: string;
    actions?: readonly RowAction<TRow>[];
    bulkActions?: readonly BulkAction<TRow>[];
    selectable?: boolean;
    loading?: boolean;
    error?: string | null;
    emptyLabel?: string;
    showSearch?: boolean;
    searchPlaceholder?: string;
    initialPage?: number;
    initialSearch?: string;
    initialSort?: TableSort | null;
    onStateChange?: (snapshot: TableStateSnapshot) => void;
  }>(),
  {
    actions: () => [],
    bulkActions: () => [],
    selectable: false,
    loading: false,
    error: null,
    emptyLabel: 'Nothing to show yet.',
    showSearch: true,
    searchPlaceholder: 'Search',
    initialPage: 1,
    initialSearch: '',
    initialSort: null,
  },
);

const dialog = useOptionalDialog();
const busyIds = ref<Set<string | number>>(new Set());

const table = useTableState<TRow>({
  rows: () => props.rows,
  columns: props.columns,
  loading: () => props.loading,
  initialPage: props.initialPage,
  initialSearch: props.initialSearch,
  initialSort: props.initialSort,
  onStateChange: (snapshot) => {
    if (props.onStateChange) {
      props.onStateChange(snapshot);
    }
  },
});

const {
  page,
  pageCount,
  total,
  search,
  sort,
  visibleRows,
  rangeStart,
  rangeEnd,
  setPage,
  setSearch,
  toggleSort,
  getRowId,
  selectedIds,
  pageSelection,
  selectedRows,
  toggleRow,
  togglePage,
  clearSelection,
} = table;

function alignClass(align: TableColumn<TRow>['align']): string {
  if (align === 'right') {
    return 'text-right';
  }
  if (align === 'center') {
    return 'text-center';
  }
  return 'text-left';
}

function visibleActions(row: TRow): RowAction<TRow>[] {
  return props.actions.filter((action) => {
    if (!action.hidden) {
      return true;
    }
    return !action.hidden(row);
  });
}

function isBusy(row: TRow): boolean {
  return busyIds.value.has(getRowId(row));
}

async function runAction(action: RowAction<TRow>, row: TRow): Promise<void> {
  if (action.disabled && action.disabled(row)) {
    return;
  }
  if (action.confirm) {
    if (!dialog) {
      console.error('[DataTable] A confirm action needs DialogProvider.');
      return;
    }
    const extra = action.confirm.props ? action.confirm.props(row) : {};
    dialog.triggerDialog(action.confirm.dialogKey, extra);
    return;
  }
  if (!action.onSelect) {
    return;
  }
  const id = getRowId(row);
  const next = new Set(busyIds.value);
  next.add(id);
  busyIds.value = next;
  try {
    await action.onSelect(row);
  } finally {
    const done = new Set(busyIds.value);
    done.delete(id);
    busyIds.value = done;
  }
}

function onSearchInput(event: Event): void {
  const target = event.target as HTMLInputElement;
  setSearch(target.value);
}
</script>

<template>
  <div class="flex w-full flex-col gap-4">
    <label v-if="showSearch" class="flex flex-col gap-2">
      <span class="text-ink-secondary text-sm font-semibold">{{
        searchPlaceholder
      }}</span>
      <input
        :value="search"
        type="search"
        class="border-border-soft bg-surface-canvas text-ink-primary w-full border px-4 py-3 text-sm lg:max-w-sm"
        :placeholder="searchPlaceholder"
        @input="onSearchInput"
      />
    </label>

    <div
      v-if="selectable && selectedIds.size > 0"
      class="flex flex-col gap-3 sm:flex-row sm:items-center"
    >
      <p class="text-ink-secondary text-sm">{{ selectedIds.size }} selected</p>
      <button
        v-for="action in bulkActions"
        :key="action.id"
        type="button"
        class="px-3 py-2 text-sm font-semibold"
        :class="action.tone === 'danger' ? 'text-danger' : 'text-ink-primary'"
        @click="action.onSelect(selectedRows)"
      >
        {{ action.label }}
      </button>
      <button
        type="button"
        class="text-ink-secondary text-sm font-semibold"
        @click="clearSelection"
      >
        Clear
      </button>
    </div>

    <p v-if="error" class="text-danger text-sm" role="alert">{{ error }}</p>

    <div v-else-if="loading" class="flex flex-col gap-2" aria-busy="true">
      <div v-for="index in 4" :key="index" class="bg-surface-sunken h-12" />
    </div>

    <p v-else-if="visibleRows.length === 0" class="text-ink-muted text-sm">
      {{ emptyLabel }}
    </p>

    <template v-else>
      <div class="hidden overflow-x-auto lg:block">
        <table class="w-full border-collapse text-left text-sm">
          <caption class="sr-only">
            {{
              caption
            }}
          </caption>
          <thead>
            <tr class="border-border-soft border-b">
              <th v-if="selectable" class="px-3 py-3" scope="col">
                <input
                  type="checkbox"
                  :checked="pageSelection === 'all'"
                  aria-label="Select every row on this page"
                  @change="togglePage"
                />
              </th>
              <th
                v-for="column in columns"
                :key="column.id"
                class="text-ink-secondary px-3 py-3 font-semibold"
                :class="alignClass(column.align)"
                scope="col"
              >
                <button
                  v-if="column.sortable"
                  type="button"
                  class="text-ink-secondary font-semibold"
                  @click="toggleSort(column.sortKey ?? column.id)"
                >
                  {{ column.header }}
                  <span
                    v-if="sort && sort.key === (column.sortKey ?? column.id)"
                  >
                    {{ sort.direction === 'asc' ? 'ascending' : 'descending' }}
                  </span>
                </button>
                <span v-else>{{ column.header }}</span>
              </th>
              <th
                v-if="actions.length > 0"
                class="text-ink-secondary px-3 py-3 text-right font-semibold"
                scope="col"
              >
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in visibleRows"
              :key="String(getRowId(row))"
              class="border-border-soft border-b"
            >
              <td v-if="selectable" class="px-3 py-3">
                <input
                  type="checkbox"
                  :checked="selectedIds.has(getRowId(row))"
                  :aria-label="'Select row ' + String(getRowId(row))"
                  @change="toggleRow(row)"
                />
              </td>
              <td
                v-for="column in columns"
                :key="column.id"
                class="text-ink-primary px-3 py-3"
                :class="alignClass(column.align)"
              >
                {{ column.cell(row) }}
              </td>
              <td v-if="actions.length > 0" class="px-3 py-3">
                <div class="flex justify-end gap-2">
                  <button
                    v-for="action in visibleActions(row)"
                    :key="action.id"
                    type="button"
                    class="px-3 py-2 text-sm font-semibold"
                    :class="
                      action.tone === 'danger'
                        ? 'text-danger'
                        : 'text-ink-primary'
                    "
                    :disabled="
                      (action.disabled && action.disabled(row)) || isBusy(row)
                    "
                    @click="runAction(action, row)"
                  >
                    {{ isBusy(row) ? 'Working' : action.label }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="flex flex-col gap-3 lg:hidden">
        <article
          v-for="row in visibleRows"
          :key="String(getRowId(row))"
          class="border-border-soft bg-surface-raised flex flex-col gap-3 border p-4"
        >
          <label
            v-if="selectable"
            class="text-ink-secondary flex items-center gap-2 text-sm"
          >
            <input
              type="checkbox"
              :checked="selectedIds.has(getRowId(row))"
              @change="toggleRow(row)"
            />
            Select
          </label>
          <div
            v-for="column in columns"
            :key="column.id"
            class="flex items-baseline justify-between gap-4"
          >
            <span class="text-ink-muted text-sm">{{ column.header }}</span>
            <span class="text-ink-primary text-sm">{{ column.cell(row) }}</span>
          </div>
          <div v-if="actions.length > 0" class="flex flex-col gap-2">
            <button
              v-for="action in visibleActions(row)"
              :key="action.id"
              type="button"
              class="border-border-inked text-ink-primary border px-3 py-2 text-sm font-semibold"
              :disabled="
                (action.disabled && action.disabled(row)) || isBusy(row)
              "
              @click="runAction(action, row)"
            >
              {{ action.label }}
            </button>
          </div>
        </article>
      </div>

      <div
        class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between"
      >
        <p class="text-ink-muted text-sm">
          {{ rangeStart }} to {{ rangeEnd }} of {{ total }}
        </p>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="border-border-soft text-ink-primary disabled:text-ink-disabled border px-3 py-2 text-sm font-semibold"
            :disabled="page <= 1"
            @click="setPage(page - 1)"
          >
            Previous
          </button>
          <p class="text-ink-secondary text-sm">{{ page }} / {{ pageCount }}</p>
          <button
            type="button"
            class="border-border-soft text-ink-primary disabled:text-ink-disabled border px-3 py-2 text-sm font-semibold"
            :disabled="page >= pageCount"
            @click="setPage(page + 1)"
          >
            Next
          </button>
        </div>
      </div>
    </template>
  </div>
</template>
