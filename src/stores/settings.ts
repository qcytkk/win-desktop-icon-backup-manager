import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

import { setLanguage } from '../i18n'
import type { AppSettings, SettingsView } from '../types'

const DEFAULTS: AppSettings = {
  theme: 'light',
  language: 'zh-CN',
  fontSize: 'normal',
  storageMode: 'documents',
  customPath: '',
  autoStart: true,
  contextMenu: true,
}

export const useSettingsStore = defineStore('settings', {
  state: () => ({
    loaded: false,
    settings: { ...DEFAULTS } as AppSettings,
    paths: { documents: '', app: '', current: '' },
  }),
  actions: {
    applyAppearance() {
      document.documentElement.setAttribute('data-theme', this.settings.theme)
      document.documentElement.setAttribute('data-font-size', this.settings.fontSize)
      setLanguage(this.settings.language)
    },
    applyView(view: SettingsView) {
      this.settings = {
        theme: view.theme,
        language: view.language,
        fontSize: view.fontSize,
        storageMode: view.storageMode,
        customPath: view.customPath,
        autoStart: view.autoStart,
        contextMenu: view.contextMenu,
      }
      this.paths = view.paths
      this.applyAppearance()
    },
    async init() {
      const view = await invoke<SettingsView>('get_settings')
      this.applyView(view)
      this.loaded = true
    },
    async persist(patch: Partial<AppSettings>) {
      const previous = { ...this.settings }
      this.settings = { ...this.settings, ...patch }
      this.applyAppearance()
      try {
        const view = await invoke<SettingsView>('save_settings', { settings: this.settings })
        this.applyView(view)
      } catch (err) {
        this.settings = previous
        this.applyAppearance()
        throw err
      }
    },
  },
})
