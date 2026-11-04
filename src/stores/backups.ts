import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

import type { BackupRecord, IconMap, ViewMode } from '../types'

export type SortKey = 'name' | 'time' | 'resolution' | 'count'
export type SortDir = 'asc' | 'desc'

const iconCache = new Map<string, IconMap>()

function readViewMode(): ViewMode {
  return localStorage.getItem('ui.viewMode') === 'list' ? 'list' : 'grid'
}

export const useBackupStore = defineStore('backups', {
  state: () => ({
    backups: [] as BackupRecord[],
    loading: false,
    creating: false,
    viewMode: readViewMode(),
    sortKey: 'time' as SortKey,
    sortDir: 'desc' as SortDir,
    previewId: null as string | null,
    previewIcons: {} as IconMap,
    previewLoading: false,
  }),
  getters: {
    previewRecord(state): BackupRecord | null {
      if (!state.previewId) return null
      return state.backups.find((b) => b.id === state.previewId) ?? null
    },
    sorted(state): BackupRecord[] {
      const direction = state.sortDir === 'asc' ? 1 : -1
      const items = state.backups.slice()
      items.sort((a, b) => {
        let va = 0
        let vb = 0
        if (state.sortKey === 'name') {
          return a.name.localeCompare(b.name) * direction
        }
        if (state.sortKey === 'resolution') {
          va = a.display.width * a.display.height
          vb = b.display.width * b.display.height
        } else if (state.sortKey === 'count') {
          va = a.iconCount
          vb = b.iconCount
        } else {
          va = new Date(a.createdAt).getTime()
          vb = new Date(b.createdAt).getTime()
        }
        if (va === vb) {
          return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
        }
        return (va - vb) * direction
      })
      return items
    },
    lastBackupAt(state): string | null {
      if (!state.backups.length) return null
      const newest = state.backups.reduce((latest, item) =>
        new Date(item.createdAt) > new Date(latest.createdAt) ? item : latest
      )
      return newest.createdAt
    },
  },
  actions: {
    async init() {
      await this.refresh()
    },
    async refresh() {
      this.loading = true
      try {
        this.backups = await invoke<BackupRecord[]>('list_backups')
      } finally {
        this.loading = false
      }
    },
    async create(name: string) {
      if (this.creating) return null
      this.creating = true
      try {
        const record = await invoke<BackupRecord>('create_backup', { name })
        this.backups.unshift(record)
        return record
      } finally {
        this.creating = false
      }
    },
    async rename(id: string, name: string) {
      const record = await invoke<BackupRecord>('rename_backup', { id, name })
      const index = this.backups.findIndex((item) => item.id === id)
      if (index !== -1) this.backups[index] = record
      return record
    },
    async remove(id: string) {
      await invoke('delete_backup', { id })
      this.backups = this.backups.filter((item) => item.id !== id)
      if (this.previewId === id) this.closePreview()
    },
    async openPreview(id: string) {
      this.previewId = id
      if (iconCache.has(id)) {
        this.previewIcons = iconCache.get(id) ?? {}
        return
      }
      this.previewIcons = {}
      this.previewLoading = true
      try {
        const icons = await invoke<IconMap>('load_icons', { id })
        iconCache.set(id, icons)
        if (this.previewId === id) this.previewIcons = icons
      } catch {
        iconCache.set(id, {})
      } finally {
        this.previewLoading = false
      }
    },
    closePreview() {
      this.previewId = null
      this.previewIcons = {}
      this.previewLoading = false
    },
    setViewMode(mode: ViewMode) {
      this.viewMode = mode
      localStorage.setItem('ui.viewMode', mode)
    },
    setSort(key: SortKey, dir: SortDir) {
      this.sortKey = key
      this.sortDir = dir
    },
  },
})
