<script setup lang="ts">
import { computed, nextTick, reactive, ref } from 'vue'

import type { BackupRecord, IconMap } from '../types'
import { clampText, colorFor, hasPositions, initials, pct, placeIcons, type PlacedIcon } from '../utils'

const props = defineProps<{ record: BackupRecord; icons: IconMap }>()

const placed = computed(() => placeIcons(props.record))
const gridMode = computed(() => !hasPositions(props.record))
const desktopStyle = computed(
  () => ({ '--ar': (props.record.display.virtualBounds.width / props.record.display.virtualBounds.height).toFixed(4) }) as Record<string, string>
)

const tip = reactive({ show: false, text: '', left: 0, top: 0 })
const tipRef = ref<HTMLElement | null>(null)

function iconSource(item: PlacedIcon): string | undefined {
  const key = item.icon.iconKey
  if (!key) return undefined
  return props.icons[key]
}

function showTip(event: MouseEvent, item: PlacedIcon) {
  const coords = item.icon.x >= 0 && item.icon.y >= 0 ? ` · (${item.icon.x}, ${item.icon.y})` : ''
  tip.text = `${item.icon.displayName}${coords}`
  tip.show = true
  moveTip(event)
}

async function moveTip(event: MouseEvent) {
  if (!tip.show) return
  tip.left = event.clientX + 14
  tip.top = event.clientY + 16
  await nextTick()
  const el = tipRef.value
  if (!el) return
  tip.left = Math.min(tip.left, window.innerWidth - el.offsetWidth - 12)
  tip.top = Math.min(tip.top, window.innerHeight - el.offsetHeight - 12)
}

function hideTip() {
  tip.show = false
}
</script>

<template>
  <div class="desktop" :style="desktopStyle">
    <div
      v-for="item in placed"
      :key="item.icon.index"
      class="di"
      :class="{ centered: gridMode, anomaly: item.anomaly }"
      :style="{ left: pct(item.left), top: pct(item.top) }"
      @mouseenter="showTip($event, item)"
      @mousemove="moveTip($event)"
      @mouseleave="hideTip"
    >
      <span class="tile" :style="iconSource(item) ? undefined : { background: colorFor(item.icon.displayName) }">
        <img v-if="iconSource(item)" :src="iconSource(item)" alt="" draggable="false" />
        <template v-else>{{ initials(item.icon.displayName) }}</template>
      </span>
      <span class="dlabel">{{ clampText(item.icon.displayName) }}</span>
    </div>
  </div>

  <div
    ref="tipRef"
    class="tip"
    :class="{ show: tip.show }"
    :style="{ left: `${tip.left}px`, top: `${tip.top}px` }"
    aria-hidden="true"
  >
    {{ tip.text }}
  </div>
</template>
