<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'

import BackupGrid from './components/BackupGrid.vue'
import BackupList from './components/BackupList.vue'
import BackupToolbar from './components/BackupToolbar.vue'
import EmptyState from './components/EmptyState.vue'
import PreviewDialog from './components/PreviewDialog.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import ToastHost from './components/ToastHost.vue'
import { t } from './i18n'
import { useBackupStore } from './stores/backups'
import { useSettingsStore } from './stores/settings'
import { showToast } from './toast'
import { defaultBackupName } from './utils'

const settings = useSettingsStore()
const backups = useBackupStore()

const booting = ref(true)
const showSettings = ref(false)

async function boot() {
  booting.value = true
  try {
    await settings.init()
    await backups.init()
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  } finally {
    booting.value = false
  }
}

async function backupNow() {
  try {
    const record = await backups.create(defaultBackupName())
    if (record) showToast(t('toast.backedUp', { n: record.iconCount }))
  } catch (err) {
    showToast(t('toast.error', { msg: String(err) }))
  }
}

watch(
  () => settings.settings.storageMode,
  async (value, previous) => {
    if (!settings.loaded || !previous || value === previous) return
    try {
      await backups.refresh()
    } catch (err) {
      showToast(t('toast.error', { msg: String(err) }))
    }
  }
)

onMounted(boot)
</script>

<template>
  <div class="app-shell">
    <BackupToolbar @open-settings="showSettings = true" />

    <main :class="['content', booting ? '' : backups.viewMode]" :aria-label="t('list.aria')">
      <div v-if="booting || backups.loading" class="skeleton-grid">
        <div v-for="n in 3" :key="n" class="skeleton-card">
          <div class="skeleton-thumb"></div>
          <div class="skeleton-body">
            <div class="skeleton-line"></div>
            <div class="skeleton-line short"></div>
          </div>
        </div>
      </div>
      <EmptyState v-else-if="!backups.backups.length" @create="backupNow" />
      <BackupGrid v-else-if="backups.viewMode === 'grid'" />
      <BackupList v-else />
    </main>

    <PreviewDialog v-if="backups.previewId" />
    <SettingsDialog v-if="showSettings" @close="showSettings = false" />
    <ToastHost />
  </div>
</template>
