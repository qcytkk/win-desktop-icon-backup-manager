<script setup lang="ts">
import { nextTick, ref } from 'vue'

import { t } from '../i18n'
import { useBackupStore } from '../stores/backups'
import { showToast } from '../toast'
import type { BackupRecord } from '../types'
import { fmtTime } from '../utils'
import LayoutThumb from './LayoutThumb.vue'

const props = defineProps<{ record: BackupRecord }>()
const store = useBackupStore()

const editing = ref(false)
const draft = ref('')
const inputRef = ref<HTMLInputElement | null>(null)

function openPreview() {
  if (editing.value) return
  void store.openPreview(props.record.id)
}

async function startEdit(event: Event) {
  event.stopPropagation()
  draft.value = props.record.name
  editing.value = true
  await nextTick()
  inputRef.value?.focus()
  inputRef.value?.select()
}

function cancelEdit() {
  editing.value = false
}

async function commitEdit() {
  if (!editing.value) return
  editing.value = false
  const name = draft.value.trim()
  if (!name || name === props.record.name) return
  try {
    await store.rename(props.record.id, name)
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}
</script>

<template>
  <div class="row" @click="openPreview">
    <div class="mini-wrap">
      <LayoutThumb :record="record" />
    </div>
    <div class="cell-name">
      <input
        v-if="editing"
        ref="inputRef"
        v-model="draft"
        class="name-input"
        maxlength="40"
        @click.stop
        @keydown.enter.prevent="commitEdit"
        @keydown.esc.prevent="cancelEdit"
        @blur="commitEdit"
      />
      <span v-else class="name">{{ record.name }}</span>
      <button
        class="icon-btn"
        type="button"
        :aria-label="t('rename')"
        :title="t('rename')"
        @click="startEdit"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M10.8 2.6l2.6 2.6L5.6 13H3v-2.6z" />
        </svg>
      </button>
    </div>
    <span class="cell-time">{{ fmtTime(record.createdAt) }}</span>
    <span class="cell-res">{{ record.display.width }}×{{ record.display.height }}</span>
    <span class="cell-scale">{{ Math.round(record.display.scaleFactor * 100) }}%</span>
    <span class="cell-count">{{ record.iconCount }}</span>
    <div class="row-actions">
      <button class="row-open" type="button" @click.stop="openPreview">
        {{ t('card.preview') }}
      </button>
    </div>
  </div>
</template>
