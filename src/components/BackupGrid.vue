<script setup lang="ts">
import { useBackupStore } from '../stores/backups'
import BackupCard from './BackupCard.vue'

const store = useBackupStore()
</script>

<template>
  <BackupCard v-for="record in store.sorted" :key="record.id" :record="record" />
</template>
