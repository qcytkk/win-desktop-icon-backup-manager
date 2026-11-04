<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'

import { trapTab } from '../focus'
import { LANGUAGES, t } from '../i18n'
import { useSettingsStore } from '../stores/settings'
import { showToast } from '../toast'
import type { FontSize, StorageMode, ThemeMode } from '../types'

const emit = defineEmits<{ close: [] }>()
const store = useSettingsStore()

const dialogRef = ref<HTMLElement | null>(null)
const closeRef = ref<HTMLButtonElement | null>(null)
const switchRef = ref<HTMLElement | null>(null)
const thumbStyle = reactive({ left: '2px', width: '0px' })
const themeStatus = ref('')

const FONT_SIZES: Array<{ value: FontSize; labelKey: string }> = [
  { value: 'small', labelKey: 'font.small' },
  { value: 'normal', labelKey: 'font.standard' },
  { value: 'large', labelKey: 'font.large' },
  { value: 'xlarge', labelKey: 'font.xlarge' },
]

const STORAGE_MODES: StorageMode[] = ['documents', 'app', 'custom']

const displayPath = computed(() => {
  if (store.settings.storageMode === 'custom' && !store.settings.customPath) {
    return t('settings.customEmpty')
  }
  return store.paths.current
})

function updateThumb() {
  const root = switchRef.value
  if (!root) return
  const active = root.querySelector<HTMLElement>('.switch-opt[aria-checked="true"]')
  if (!active) return
  thumbStyle.left = `${active.offsetLeft - root.clientLeft}px`
  thumbStyle.width = `${active.offsetWidth}px`
}

async function changeTheme(theme: ThemeMode) {
  if (theme === store.settings.theme) return
  try {
    await store.persist({ theme })
    themeStatus.value = t(`themeStatus.${theme}`)
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function changeLanguage(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  const label = LANGUAGES.find((item) => item.value === value)?.label ?? value
  try {
    await store.persist({ language: value })
    showToast(t('toast.langChanged', { lang: label }))
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function changeFont(size: FontSize) {
  if (size === store.settings.fontSize) return
  const option = FONT_SIZES.find((item) => item.value === size)
  try {
    await store.persist({ fontSize: size })
    showToast(t('toast.fontChanged', { size: t(option ? option.labelKey : 'font.standard') }))
    await nextTick()
    updateThumb()
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function pickFolder(): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
    defaultPath: store.paths.current,
    title: t('settings.storageSection'),
  })
  const value = Array.isArray(result) ? result[0] : result
  return value || null
}

async function saveStorage(mode: StorageMode, path?: string) {
  try {
    await store.persist({
      storageMode: mode,
      ...(path ? { customPath: path } : {}),
    })
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

async function selectStorage(mode: StorageMode) {
  if (mode === 'custom') {
    const path = await pickFolder()
    if (path) await saveStorage('custom', path)
    return
  }
  if (mode !== store.settings.storageMode) await saveStorage(mode)
}

async function changeCustomFolder() {
  const path = await pickFolder()
  if (path) await saveStorage('custom', path)
}

function handleKey(event: KeyboardEvent) {
  if (event.key === 'Tab') {
    trapTab(event, dialogRef.value)
    return
  }
  if (event.key === 'Escape') {
    const target = event.target as HTMLElement | null
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return
    event.preventDefault()
    emit('close')
  }
}

watch(
  () => store.settings.storageMode,
  async () => {
    await nextTick()
    updateThumb()
  }
)

watch(
  () => store.settings.fontSize,
  async () => {
    await nextTick()
    updateThumb()
  }
)

watch(
  () => store.settings.language,
  async () => {
    await nextTick()
    updateThumb()
  }
)

onMounted(async () => {
  window.addEventListener('keydown', handleKey)
  window.addEventListener('resize', updateThumb)
  await nextTick()
  updateThumb()
  closeRef.value?.focus()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKey)
  window.removeEventListener('resize', updateThumb)
})
</script>

<template>
  <div class="settings-overlay" @click.self="emit('close')">
    <div ref="dialogRef" class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settingsTitle">
      <header class="settings-head">
        <div>
          <h2 id="settingsTitle" class="settings-title">{{ t('settings.title') }}</h2>
        </div>
        <button ref="closeRef" class="close-btn" type="button" :aria-label="t('settings.close')" @click="emit('close')">
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" /></svg>
        </button>
      </header>

      <div class="settings-body">
        <section class="settings-section" aria-labelledby="langLabel">
          <div class="section-head">
            <h3 id="langLabel">{{ t('settings.langSection') }}</h3>
            <p>{{ t('settings.langDesc') }}</p>
          </div>
          <div class="setting-row">
            <div class="row-label">
              <b>{{ t('settings.langLabel') }}</b>
              <small>{{ t('settings.langHint') }}</small>
            </div>
            <div class="row-control">
              <div class="select-wrap">
                <select
                  :value="store.settings.language"
                  :aria-label="t('settings.langLabel')"
                  @change="changeLanguage"
                >
                  <option v-for="lang in LANGUAGES" :key="lang.value" :value="lang.value">
                    {{ lang.label }}
                  </option>
                </select>
                <svg class="chev" viewBox="0 0 12 12" aria-hidden="true">
                  <path d="M2.5 4.5l3.5 3.5 3.5-3.5" />
                </svg>
              </div>
            </div>
          </div>
        </section>

        <section class="settings-section" aria-labelledby="themeLabel">
          <div class="section-head">
            <h3 id="themeLabel">{{ t('settings.themeSection') }}</h3>
            <p>{{ t('settings.themeDesc') }}</p>
          </div>
          <div class="theme-options" role="radiogroup" :aria-labelledby="'themeLabel'">
            <button
              class="theme-card"
              type="button"
              role="radio"
              :aria-checked="store.settings.theme === 'light'"
              @click="changeTheme('light')"
            >
              <span class="theme-swatch light" aria-hidden="true"><i></i><em></em></span>
              <span class="theme-meta">
                <span class="theme-info">
                  <b>{{ t('theme.light') }}</b>
                  <small>{{ t('theme.lightDesc') }}</small>
                </span>
                <span class="theme-check" aria-hidden="true">✓</span>
              </span>
            </button>
            <button
              class="theme-card"
              type="button"
              role="radio"
              :aria-checked="store.settings.theme === 'dark'"
              @click="changeTheme('dark')"
            >
              <span class="theme-swatch dark" aria-hidden="true"><i></i><em></em></span>
              <span class="theme-meta">
                <span class="theme-info">
                  <b>{{ t('theme.dark') }}</b>
                  <small>{{ t('theme.darkDesc') }}</small>
                </span>
                <span class="theme-check" aria-hidden="true">✓</span>
              </span>
            </button>
          </div>
        </section>

        <section class="settings-section" aria-labelledby="fontLabel">
          <div class="section-head">
            <h3 id="fontLabel">{{ t('settings.fontSection') }}</h3>
            <p>{{ t('settings.fontDesc') }}</p>
          </div>
          <div class="setting-row">
            <div class="row-label">
              <b>{{ t('settings.scaleLabel') }}</b>
              <small>{{ t('settings.scaleHint') }}</small>
            </div>
            <div class="font-options" role="group" :aria-label="t('settings.fontAria')">
              <button
                v-for="size in FONT_SIZES"
                :key="size.value"
                class="font-opt"
                :class="{ active: store.settings.fontSize === size.value }"
                type="button"
                :aria-pressed="store.settings.fontSize === size.value"
                @click="changeFont(size.value)"
              >
                {{ t(size.labelKey) }}
              </button>
            </div>
          </div>
        </section>

        <section class="settings-section" aria-labelledby="storageLabel">
          <div class="section-head">
            <h3 id="storageLabel">{{ t('settings.storageSection') }}</h3>
            <p>{{ t('settings.storageDesc') }}</p>
          </div>
          <div ref="switchRef" class="storage-switch" role="radiogroup" :aria-labelledby="'storageLabel'">
            <span class="switch-thumb" aria-hidden="true" :style="thumbStyle"></span>
            <button
              v-for="mode in STORAGE_MODES"
              :key="mode"
              class="switch-opt"
              type="button"
              role="radio"
              :aria-checked="store.settings.storageMode === mode"
              @click="selectStorage(mode)"
            >
              {{ t(`storage.${mode}`) }}
            </button>
          </div>
          <div class="storage-path">
            <div class="storage-path-head">
              <span class="storage-path-label">{{ t('settings.dirLabel') }}</span>
              <span class="storage-path-hint">{{ t(`storage.hint.${store.settings.storageMode}`) }}</span>
            </div>
            <div class="storage-path-row">
              <input class="path-input" type="text" readonly :value="displayPath" :aria-label="t('settings.dirLabel')" />
              <button
                v-if="store.settings.storageMode === 'custom'"
                class="placeholder-btn"
                type="button"
                @click="changeCustomFolder"
              >
                {{ t('settings.change') }}
              </button>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
  <span class="sr-only" role="status" aria-live="polite">{{ themeStatus }}</span>
</template>
