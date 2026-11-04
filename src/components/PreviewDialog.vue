<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import { trapTab } from '../focus'
import { t } from '../i18n'
import { useBackupStore } from '../stores/backups'
import { showToast } from '../toast'
import type { ApplyResult, DisplayInfo } from '../types'
import { fmtFull, hasPositions } from '../utils'
import LayoutCanvas from './LayoutCanvas.vue'

const store = useBackupStore()
const record = computed(() => store.previewRecord)

const dialogRef = ref<HTMLElement | null>(null)
const confirmRef = ref<HTMLElement | null>(null)
const scaleRef = ref<HTMLElement | null>(null)
const closeRef = ref<HTMLButtonElement | null>(null)
const inputRef = ref<HTMLInputElement | null>(null)

const confirmOpen = ref(false)
const scaleOpen = ref(false)
const scaleMode = ref<'current' | 'backup'>('current')
const currentDisplay = ref<DisplayInfo | null>(null)
const applying = ref(false)
const editing = ref(false)
const draft = ref('')

const hasCoords = computed(() => (record.value ? hasPositions(record.value) : false))
const confirmParts = computed(() => {
  const raw = t('confirm.text', { name: '\u0000' })
  const index = raw.indexOf('\u0000')
  return [raw.slice(0, index), raw.slice(index + 1)]
})

const currentPercent = computed(() =>
  currentDisplay.value ? Math.round(currentDisplay.value.scaleFactor * 100) : 0
)
const backupPercent = computed(() =>
  record.value ? Math.round(record.value.display.scaleFactor * 100) : 0
)
const scaleMeta = computed(() => {
  if (!record.value || !currentDisplay.value) return ''
  return t('scale.meta', {
    cw: currentDisplay.value.width,
    ch: currentDisplay.value.height,
    cs: currentPercent.value,
    bw: record.value.display.width,
    bh: record.value.display.height,
    bs: backupPercent.value,
  })
})

async function startRename() {
  if (!record.value) return
  draft.value = record.value.name
  editing.value = true
  await nextTick()
  inputRef.value?.focus()
  inputRef.value?.select()
}

function cancelRename() {
  editing.value = false
}

async function commitRename() {
  if (!editing.value || !record.value) return
  editing.value = false
  const name = draft.value.trim()
  if (!name || name === record.value.name) return
  try {
    await store.rename(record.value.id, name)
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function useLayout() {
  if (!record.value || applying.value) return
  if (!hasCoords.value) {
    showToast(t('toast.noPosition'))
    return
  }
  try {
    const display = await invoke<DisplayInfo>('get_current_display')
    currentDisplay.value = display
    if (Math.round(display.scaleFactor * 100) !== backupPercent.value) {
      scaleMode.value = 'current'
      scaleOpen.value = true
    } else {
      await doApply('current')
    }
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function doApply(mode: 'current' | 'backup') {
  const target = record.value
  if (!target || applying.value) return
  applying.value = true
  scaleOpen.value = false
  try {
    const result = await invoke<ApplyResult>('apply_backup', {
      id: target.id,
      scaleMode: mode,
    })
    store.closePreview()
    let message = t('toast.applied', { n: result.matched })
    if (result.missing.length) {
      message += ` · ${t('toast.missing', { n: result.missing.length })}`
    }
    if (result.newPlaced) {
      message += ` · ${t('toast.added', { n: result.newPlaced })}`
    }
    if (result.autoArrangeDisabled) {
      message += ` · ${t('toast.autoArrange')}`
    }
    showToast(message)
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  } finally {
    applying.value = false
  }
}

async function removeBackup() {
  if (!record.value) return
  const current = record.value
  confirmOpen.value = false
  try {
    await store.remove(current.id)
    showToast(t('toast.deleted', { name: current.name }))
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

function handleKey(event: KeyboardEvent) {
  if (event.key === 'Tab') {
    const container = scaleOpen.value
      ? scaleRef.value
      : confirmOpen.value
        ? confirmRef.value
        : dialogRef.value
    trapTab(event, container)
    return
  }
  if (event.key !== 'Escape') return
  const target = event.target as HTMLElement | null
  if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return
  event.preventDefault()
  if (scaleOpen.value) {
    scaleOpen.value = false
    return
  }
  if (confirmOpen.value) {
    confirmOpen.value = false
    return
  }
  store.closePreview()
}

watch(confirmOpen, async (visible) => {
  if (!visible) return
  await nextTick()
  confirmRef.value?.querySelector<HTMLElement>('.btn-ghost')?.focus()
})

watch(scaleOpen, async (visible) => {
  if (!visible) return
  await nextTick()
  scaleRef.value?.querySelector<HTMLElement>('.scale-option')?.focus()
})

onMounted(async () => {
  window.addEventListener('keydown', handleKey)
  await nextTick()
  closeRef.value?.focus()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKey)
})
</script>

<template>
  <div v-if="record" class="overlay" @click.self="store.closePreview()">
    <div
      ref="dialogRef"
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="pvName"
      aria-describedby="pvMeta"
    >
      <header class="dlg-head">
        <div>
          <div class="dlg-title-row">
            <input
              v-if="editing"
              ref="inputRef"
              v-model="draft"
              class="name-input"
              maxlength="40"
              @keydown.enter.prevent="commitRename"
              @keydown.esc.prevent="cancelRename"
              @blur="commitRename"
            />
            <h2 v-else id="pvName">{{ record.name }}</h2>
            <button
              class="icon-btn"
              type="button"
              :aria-label="t('rename')"
              :title="t('rename')"
              @click="startRename"
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path d="M10.8 2.6l2.6 2.6L5.6 13H3v-2.6z" />
              </svg>
            </button>
          </div>
          <p id="pvMeta" class="dlg-meta">
            {{ fmtFull(record.createdAt) }} · {{ record.display.width }}×{{ record.display.height }}
            @{{ Math.round(record.display.scaleFactor * 100) }}% ·
            {{ t('card.icons', { n: record.iconCount }) }}
          </p>
        </div>
        <div class="dlg-actions">
          <button
            ref="closeRef"
            class="close-btn"
            type="button"
            :aria-label="t('preview.close')"
            @click="store.closePreview()"
          >
            <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
          </button>
        </div>
      </header>

      <div class="dlg-body">
        <p v-if="!hasCoords" class="warning-bar">{{ t('preview.noPosition') }}</p>
        <LayoutCanvas :record="record" :icons="store.previewIcons" />
        <p class="dlg-foot">{{ t('preview.hint') }}</p>
      </div>

      <footer class="dlg-foot-bar">
        <button class="btn-danger-ghost" type="button" @click="confirmOpen = true">
          {{ t('preview.delete') }}
        </button>
        <button
          class="btn-primary btn-sm"
          :class="{ loading: applying }"
          type="button"
          :disabled="applying || !hasCoords"
          :title="hasCoords ? '' : t('toast.noPosition')"
          @click="useLayout"
        >
          <span class="spinner" aria-hidden="true"></span>
          {{ t(applying ? 'scale.applying' : 'preview.use') }}
        </button>
      </footer>
    </div>

    <div v-if="confirmOpen" class="confirm-overlay" @click.self="confirmOpen = false">
      <div ref="confirmRef" class="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirmTitle">
        <h2 id="confirmTitle" class="confirm-title">{{ t('confirm.title') }}</h2>
        <p class="confirm-text">
          {{ confirmParts[0] }}<b>{{ record.name }}</b>{{ confirmParts[1] }}
        </p>
        <div class="confirm-actions">
          <button class="btn-ghost" type="button" @click="confirmOpen = false">
            {{ t('confirm.cancel') }}
          </button>
          <button class="btn-danger" type="button" @click="removeBackup">
            {{ t('confirm.delete') }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="scaleOpen" class="scale-overlay" @click.self="scaleOpen = false">
      <div
        ref="scaleRef"
        class="scale-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="scaleTitle"
      >
        <h2 id="scaleTitle" class="confirm-title">{{ t('scale.title') }}</h2>
        <p class="confirm-text">{{ t('scale.subtitle') }}</p>
        <p class="scale-meta">{{ scaleMeta }}</p>
        <div class="scale-options" role="radiogroup" :aria-label="t('scale.title')">
          <button
            class="scale-option"
            type="button"
            role="radio"
            :aria-checked="scaleMode === 'current'"
            @click="scaleMode = 'current'"
          >
            <b>{{ t('scale.current', { n: currentPercent }) }}</b>
            <small>{{ t('scale.currentHint') }}</small>
            <span class="theme-check" aria-hidden="true">✓</span>
          </button>
          <button
            class="scale-option"
            type="button"
            role="radio"
            :aria-checked="scaleMode === 'backup'"
            @click="scaleMode = 'backup'"
          >
            <b>{{ t('scale.backup', { n: backupPercent }) }}</b>
            <small>{{ t('scale.backupHint') }}</small>
            <span class="theme-check" aria-hidden="true">✓</span>
          </button>
        </div>
        <p class="scale-note">{{ t('scale.note') }}</p>
        <div class="confirm-actions">
          <button class="btn-ghost" type="button" @click="scaleOpen = false">
            {{ t('confirm.cancel') }}
          </button>
          <button
            class="btn-primary btn-sm"
            :class="{ loading: applying }"
            type="button"
            :disabled="applying"
            @click="doApply(scaleMode)"
          >
            <span class="spinner" aria-hidden="true"></span>
            {{ t(applying ? 'scale.applying' : 'scale.apply') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
