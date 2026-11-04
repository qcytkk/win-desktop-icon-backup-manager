<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'

import { t } from '../i18n'
import { useBackupStore, type SortKey } from '../stores/backups'
import { showToast } from '../toast'
import { defaultBackupName, fmtFull } from '../utils'

const emit = defineEmits<{ 'open-settings': [] }>()
const store = useBackupStore()

const SORT_OPTIONS: Array<{ value: SortKey; labelKey: string }> = [
  { value: 'name', labelKey: 'sort.name' },
  { value: 'time', labelKey: 'sort.time' },
  { value: 'resolution', labelKey: 'sort.resolution' },
  { value: 'count', labelKey: 'sort.count' },
]

const sortOpen = ref(false)
const triggerRef = ref<HTMLButtonElement | null>(null)
const menuRef = ref<HTMLElement | null>(null)
const menuStyle = ref<Record<string, string>>({})

const sortValue = computed(() => {
  const option = SORT_OPTIONS.find((item) => item.value === store.sortKey)
  return t(option ? option.labelKey : 'sort.time')
})
const ascending = computed(() => store.sortDir === 'asc')
const sortHint = computed(() =>
  t(ascending.value ? 'sort.ariaAsc' : 'sort.ariaDesc')
)
const lastBackup = computed(() => (store.lastBackupAt ? fmtFull(store.lastBackupAt) : '—'))
const sortStatus = computed(() =>
  t('sort.status', {
    option: sortValue.value,
    dir: t(ascending.value ? 'sort.dir.asc' : 'sort.dir.desc'),
  })
)

async function toggleSort() {
  if (sortOpen.value) {
    closeSort(true)
    return
  }
  const rect = triggerRef.value?.getBoundingClientRect()
  if (rect) {
    menuStyle.value = {
      top: `${rect.bottom + 4}px`,
      left: `${Math.max(8, Math.min(rect.left, window.innerWidth - 176))}px`,
    }
  }
  sortOpen.value = true
  await nextTick()
  const selected = menuRef.value?.querySelector<HTMLElement>('.dd-item.selected')
  selected?.focus()
}

function closeSort(restoreFocus = false) {
  if (!sortOpen.value) return
  sortOpen.value = false
  if (restoreFocus) triggerRef.value?.focus()
}

function chooseSort(value: SortKey) {
  store.setSort(value, store.sortDir)
  closeSort(true)
}

function toggleDirection() {
  store.setSort(store.sortKey, ascending.value ? 'desc' : 'asc')
}

function handleMenuKeys(event: KeyboardEvent) {
  const items = Array.from(menuRef.value?.querySelectorAll<HTMLElement>('.dd-item') ?? [])
  const index = items.indexOf(document.activeElement as HTMLElement)
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    items[Math.min(index + 1, items.length - 1)]?.focus()
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    items[Math.max(index - 1, 0)]?.focus()
  } else if (event.key === 'Home') {
    event.preventDefault()
    items[0]?.focus()
  } else if (event.key === 'End') {
    event.preventDefault()
    items[items.length - 1]?.focus()
  } else if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    closeSort(true)
  }
}

async function backupNow() {
  if (store.creating) return
  try {
    const record = await store.create(defaultBackupName())
    if (record) showToast(t('toast.backedUp', { n: record.iconCount }))
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}
</script>

<template>
  <section class="toolbar">
    <div>
      <div class="h-row">
        <h1>{{ t('toolbar.heading') }}</h1>
        <span class="count">{{ store.backups.length }}</span>
      </div>
      <p class="sub">
        <span>{{ t('toolbar.lastBackupPrefix') }}</span>
        <b>{{ lastBackup }}</b>
      </p>
    </div>

    <div class="actions">
      <button
        class="btn-primary"
        :class="{ loading: store.creating }"
        type="button"
        :disabled="store.creating"
        @click="backupNow"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
        <span class="spinner" aria-hidden="true"></span>
        <span class="btn-label">{{ t(store.creating ? 'toolbar.collecting' : 'toolbar.backup') }}</span>
      </button>

      <div class="pill-group" role="group" :aria-label="t('sort.label')">
        <span class="group-label" aria-hidden="true">{{ t('sort.label') }}</span>
        <button
          ref="triggerRef"
          class="dd-trigger"
          type="button"
          aria-haspopup="listbox"
          :aria-expanded="sortOpen"
          @click.stop="toggleSort"
        >
          <span class="dd-value">{{ sortValue }}</span>
          <span class="arrow" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" /></svg>
          </span>
        </button>
        <button
          class="dir-btn"
          type="button"
          :title="t(ascending ? 'sort.dir.asc' : 'sort.dir.desc')"
          :aria-label="sortHint"
          @click="toggleDirection"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path :d="ascending ? 'M12 19V5M5 12l7-7 7 7' : 'M12 5v14M19 12l-7 7-7-7'" />
          </svg>
        </button>
      </div>
      <span class="sr-only" role="status" aria-live="polite">{{ sortStatus }}</span>

      <div class="seg" role="group" :aria-label="t('view.groupAria')">
        <button
          type="button"
          :aria-pressed="store.viewMode === 'grid'"
          :aria-label="t('view.grid')"
          :title="t('view.grid')"
          @click="store.setViewMode('grid')"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect x="2" y="2" width="5" height="5" />
            <rect x="9" y="2" width="5" height="5" />
            <rect x="2" y="9" width="5" height="5" />
            <rect x="9" y="9" width="5" height="5" />
          </svg>
          <span>{{ t('view.grid') }}</span>
        </button>
        <button
          type="button"
          :aria-pressed="store.viewMode === 'list'"
          :aria-label="t('view.list')"
          :title="t('view.list')"
          @click="store.setViewMode('list')"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2 3h1M5 3h9M2 8h1M5 8h9M2 13h1M5 13h9" />
          </svg>
          <span>{{ t('view.list') }}</span>
        </button>
      </div>

      <button
        id="settingsBtn"
        class="icon-btn"
        type="button"
        :aria-label="t('settings.title')"
        :title="t('settings.title')"
        @click="emit('open-settings')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path
            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.09a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.09a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
          />
        </svg>
      </button>
    </div>
  </section>

  <Teleport to="body">
    <div v-if="sortOpen" class="dd-mask" @click="closeSort(true)"></div>
    <div
      v-if="sortOpen"
      ref="menuRef"
      class="dd-menu"
      role="listbox"
      :aria-label="t('sort.label')"
      :style="menuStyle"
      @keydown="handleMenuKeys"
    >
      <button
        v-for="option in SORT_OPTIONS"
        :key="option.value"
        class="dd-item"
        :class="{ selected: option.value === store.sortKey }"
        type="button"
        role="option"
        :aria-selected="option.value === store.sortKey"
        @click="chooseSort(option.value)"
      >
        <span>{{ t(option.labelKey) }}</span>
        <span class="tick" aria-hidden="true">✓</span>
      </button>
    </div>
  </Teleport>
</template>
