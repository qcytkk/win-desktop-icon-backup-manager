<script setup lang="ts">
import { t } from '../i18n'
import { useBackupStore } from '../stores/backups'
import BackupRow from './BackupRow.vue'

const store = useBackupStore()
</script>

<template>
  <div class="list-head">
    <span></span>
    <span>{{ t('list.name') }}</span>
    <span>{{ t('list.time') }}</span>
    <span>{{ t('list.resolution') }}</span>
    <span>{{ t('list.scale') }}</span>
    <span>{{ t('list.count') }}</span>
    <span>{{ t('list.actions') }}</span>
  </div>
  <BackupRow v-for="record in store.sorted" :key="record.id" :record="record" />
</template>
