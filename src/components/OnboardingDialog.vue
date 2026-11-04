<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import guideImage from '../assets/onboarding-desktop-icons.png'
import { trapTab } from '../focus'
import { t } from '../i18n'
import { showToast } from '../toast'

const emit = defineEmits<{ close: [] }>()

const dialogRef = ref<HTMLElement | null>(null)
const closeRef = ref<HTMLButtonElement | null>(null)

async function openIconSettings() {
  try {
    await invoke('open_desktop_icon_settings')
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

function handleKey(event: KeyboardEvent) {
  if (event.key === 'Tab') {
    trapTab(event, dialogRef.value)
    return
  }
  if (event.key === 'Escape') {
    event.preventDefault()
    emit('close')
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKey)
  closeRef.value?.focus()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKey)
})
</script>

<template>
  <div class="guide-overlay" @click.self="emit('close')">
    <div ref="dialogRef" class="guide-dialog" role="dialog" aria-modal="true" aria-labelledby="guideTitle">
      <header class="guide-head">
        <h2 id="guideTitle" class="guide-title">{{ t('guide.title') }}</h2>
        <button
          ref="closeRef"
          class="close-btn"
          type="button"
          :aria-label="t('guide.close')"
          @click="emit('close')"
        >
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
        </button>
      </header>

      <ol class="guide-steps">
        <li>
          {{ t('guide.step1') }}
          <small class="guide-warning">{{ t('guide.warning') }}</small>
        </li>
        <li>{{ t('guide.step2') }}</li>
      </ol>

      <img class="guide-image" :src="guideImage" :alt="t('guide.step1')" />

      <div class="guide-actions">
        <button class="btn-primary guide-open" type="button" @click="openIconSettings">
          {{ t('guide.open') }}
        </button>
      </div>
    </div>
  </div>
</template>
