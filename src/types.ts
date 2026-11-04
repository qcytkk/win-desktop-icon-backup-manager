export type ViewMode = 'grid' | 'list'
export type IconKind = 'file' | 'folder' | 'shortcut' | 'url' | 'system'
export type LayoutSource = 'folder_view' | 'icon_layouts' | 'name_only'

export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export interface MonitorInfo {
  x: number
  y: number
  width: number
  height: number
  primary: boolean
}

export interface DisplayInfo {
  width: number
  height: number
  dpi: number
  scaleFactor: number
  virtualBounds: Rect
  monitors: MonitorInfo[]
}

export interface IconItem {
  index: number
  rawName: string
  displayName: string
  kind: IconKind
  path: string
  x: number
  y: number
  iconKey: string | null
}

export interface BackupRecord {
  id: string
  name: string
  createdAt: string
  display: DisplayInfo
  iconCount: number
  layoutSource: LayoutSource
  warnings: string[]
  icons: IconItem[]
}

export type StorageMode = 'documents' | 'app' | 'custom'
export type ThemeMode = 'light' | 'dark'
export type FontSize = 'small' | 'normal' | 'large' | 'xlarge'

export interface AppSettings {
  theme: ThemeMode
  language: string
  fontSize: FontSize
  storageMode: StorageMode
  customPath: string
  autoStart: boolean
  contextMenu: boolean
}

export interface PathsInfo {
  documents: string
  app: string
  current: string
}

export interface SettingsView extends AppSettings {
  paths: PathsInfo
}

export interface IconMap {
  [key: string]: string
}

export interface ApplyResult {
  matched: number
  missing: string[]
  newPlaced: number
  autoArrangeDisabled: boolean
  registrySynced: boolean
  warnings: string[]
}
