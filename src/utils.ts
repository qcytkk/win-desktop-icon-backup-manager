import type { BackupRecord, IconItem } from './types'
import { t } from './i18n'

const COLORS = ['#1A5FBF', '#1F6FD1', '#0E7C85', '#6B43C0', '#A32E3C', '#96620F', '#A03D77', '#4C5563']

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n)
}

export function fmtTime(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return '—'
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function fmtFull(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return '—'
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function nowStamp(): string {
  const d = new Date()
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function defaultBackupName(): string {
  return t('backup.default', { time: nowStamp() })
}

export function colorFor(name: string): string {
  let h = 0
  for (let i = 0; i < name.length; i++) {
    h = (h * 31 + name.charCodeAt(i)) >>> 0
  }
  return COLORS[h % COLORS.length]
}

export function initials(name: string): string {
  if (/^[\u4e00-\u9fa5]/.test(name)) return name.slice(0, 1)
  const parts = name.trim().split(/\s+/)
  const a = parts[0] ? parts[0][0] : ''
  const b = parts[1] ? parts[1][0] : ''
  return (a + b).toUpperCase()
}

export interface PlacedIcon {
  icon: IconItem
  left: number
  top: number
  anomaly: boolean
}

export function hasPositions(record: BackupRecord): boolean {
  if (record.layoutSource === 'name_only') return false
  return record.icons.some((icon) => icon.x >= 0 && icon.y >= 0)
}

export function placeIcons(record: BackupRecord): PlacedIcon[] {
  const bounds = record.display.virtualBounds
  const width = bounds.width || record.display.width || 1
  const height = bounds.height || record.display.height || 1
  const count = record.icons.length
  const gridMode = !hasPositions(record)
  const aspect = width / height

  if (gridMode) {
    const cols = Math.max(1, Math.ceil(Math.sqrt(Math.max(count, 1) * aspect)))
    const rows = Math.max(1, Math.ceil(count / cols))
    return record.icons.map((icon, index) => {
      const col = index % cols
      const row = Math.floor(index / cols)
      return {
        icon,
        left: (col + 0.5) / cols,
        top: (row + 0.5) / rows,
        anomaly: false,
      }
    })
  }

  return record.icons.map((icon) => {
    let anomaly = false
    let x = icon.x - bounds.x
    let y = icon.y - bounds.y
    if (x < 0 || y < 0) {
      anomaly = true
      x = Math.max(0, x)
      y = Math.max(0, y)
    }
    if (x > width) {
      anomaly = true
      x = width
    }
    if (y > height) {
      anomaly = true
      y = height
    }
    return {
      icon,
      left: x / width,
      top: y / height,
      anomaly,
    }
  })
}

export function pct(value: number): string {
  return `${(value * 100).toFixed(3)}%`
}

export function clampText(name: string, length = 8): string {
  const chars = Array.from(name)
  if (chars.length <= length) return name
  return `${chars.slice(0, length).join('')}…`
}
