<script setup lang="ts">
import { t } from '../i18n'

defineEmits<{ create: [] }>()
</script>

<template>
  <div class="empty-state">
    <div class="empty-art" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
    <h2>{{ t('empty.title') }}</h2>
    <p>{{ t('empty.desc') }}</p>
    <button class="btn-primary" type="button" @click="$emit('create')">
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
      {{ t('empty.action') }}
    </button>
  </div>
</template>
