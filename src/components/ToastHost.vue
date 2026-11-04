<script setup lang="ts">
import { toastState } from '../toast'
</script>

<template>
  <div class="toast" :class="{ show: toastState.visible }" role="status" aria-live="polite">
    {{ toastState.message }}
  </div>
</template>
