# 坐标采集预研结果（P0）

**结论：路径 A（Shell `IFolderView`）成功。本期 `layoutSource = "folder_view"`。**

## 测试环境

| 项目 | 实测值 |
|---|---|
| 系统 | Windows 10 Pro for Workstations 22H2，Build 19045.6466 |
| 主屏 | 2560 × 1440 |
| 缩放 | 100%（`AppliedDPI = 96`，`GetDpiForMonitor = 96`） |
| 桌面图标数 | 131（与注册表 `IconLayouts` 名称表条目数一致） |
| WebView2 | 156.0.4314.58 |

## 采集结果

- 通过 `IShellWindows::FindWindowSW(SWC_DESKTOP)` → `IServiceProvider` → `IShellBrowser` → `IShellView` → `IFolderView` 链路，成功读取全部 131 个桌面项。
- `IFolderView::ItemCount(SVGIO_ALLVIEW)` 返回 131；逐项 `Item(i)` + `GetItemPosition` 取得坐标，未出现任何失败项（`warnings` 为空）。
- 坐标样例：

| 图标 | 类型 | 坐标 |
|---|---|---|
| 回收站 | system | (18, 2) |
| YuE2 Studio | shortcut | (18, 275) |
| Krita | shortcut | (18, 366) |
| LM Studio | shortcut | (18, 457) |
| Unreal Engine | shortcut | (18, 548) |

- 坐标分布：x 为 18 / 93 / 168 … 步长 75px，y 步长约 91px，符合 100% 缩放下的桌面网格。
- 图标位图：`IShellItemImageFactory::GetImage(48×48, SIIGBF_ICONONLY)` 成功输出 131 个 PNG，透明通道正常（Krita 圆形图标背景透明）。

## 注册表快照

`HKCU\Software\Microsoft\Windows\Shell\Bags\1\Desktop` 值：

| 名称 | 类型 | 字节数 |
|---|---|---|
| FFlags | REG_DWORD | 4 |
| GroupByDirection | REG_DWORD | 4 |
| GroupByKey:FMTID | REG_SZ | 78 |
| GroupByKey:PID | REG_DWORD | 4 |
| GroupView | REG_DWORD | 4 |
| IconLayouts | REG_BINARY | 11988 |
| IconNameVersion | REG_DWORD | 4 |
| IconSize | REG_DWORD | 4 |
| LogicalViewMode | REG_DWORD | 4 |
| Mode | REG_DWORD | 4 |
| Sort | REG_BINARY | 20 |

### IconLayouts 名称表结构（已修正并验证）

```
0x18  u32  条目数 = 131
0x1c  u32  保留（0）
0x20  u32  字符串字符数（含结尾 NUL）
0x24  u32  保留（0）
0x28  UTF-16LE 名称（形如 "Krita.lnk>  \0"）
...   共 131 条，名称表结束于 0x1B26（6950）
0x1B26 之后为 5038 字节坐标段（格式未公开，本期不解析）
```

名称表解析与实测数据 100% 匹配（131/131）。

## 判定

| 路径 | 结果 | 说明 |
|---|---|---|
| A. `IFolderView` | ✅ 成功 | 131/131 坐标 + 类型 + 路径 + 图标位图 |
| B. 解析 `IconLayouts` 坐标段 | 未使用 | 仅保留名称表解析，作为 A 失败时的降级来源 |
| C. `name_only` | 兜底 | A、B 均失败时，按注册表名称生成网格布局 |

## 注意事项

1. `FindWindowSW` 的 `pvarlocroot` 不能传空指针，必须传入有效的空 `VARIANT`，否则 COM 会返回 `RPC_X_NULL_REF_POINTER (0x800706F4)`。
2. 采集必须在已 `CoInitializeEx(COINIT_APARTMENTTHREADED)` 的 STA 线程中执行，且结束前 `CoUninitialize`。
3. 进程需在读取显示器信息前设置 Per-Monitor V2 DPI 感知，否则 `GetDpiForMonitor` 可能被虚拟化为 96。
4. 本机虽为 Windows 10 22H2，但注册表同样使用 `IconLayouts`（无 `ItemPos*`），与“仅 Win11 使用 IconLayouts”的初始假设不同；备份逻辑已兼容两种值。
