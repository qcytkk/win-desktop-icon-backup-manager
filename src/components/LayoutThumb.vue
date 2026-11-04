<script setup lang="ts">
import { computed } from 'vue'

import type { BackupRecord } from '../types'
import { colorFor, hasPositions, pct, placeIcons } from '../utils'

const props = defineProps<{ record: BackupRecord }>()

const placed = computed(() => placeIcons(props.record))
const gridMode = computed(() => !hasPositions(props.record))
</script>

<template>
  <div class="mini">
    <i
      v-for="item in placed"
      :key="item.icon.index"
      class="di"
      :class="{ centered: gridMode }"
      :style="{
        left: pct(item.left),
        top: pct(item.top),
        background: colorFor(item.icon.displayName),
      }"
    />
  </div>
</template>
