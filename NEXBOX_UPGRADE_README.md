# NexBox 升级包 — 自动游戏检测引擎

## 内容

```
nexbox-upgrade/
├── src-tauri/src/
│   ├── game_detector.rs    ← 新文件：自动游戏检测（60+游戏库，进程轮询，事件通知）
│   ├── game_profile.rs     ← 新文件：游戏配置持久化 + 自动优化（18项优化开关，3套预设）
│   └── lib.rs              ← 修改版：注册了2个新模块 + GameDetectorState + 12条新命令
│
├── src/pages/
│   └── GameProfiles.tsx    ← 新文件：游戏配置管理页面（当前游戏状态、添加/删除配置、自动优化开关）
│
├── src/components/
│   └── GameDetectorBadge.tsx ← 新文件：顶部栏游戏检测徽标（显示当前游戏名+一键开关）
│
└── GAME_DETECTOR_INTEGRATION.md  ← 完整集成指南（12步，5分钟完成）
```

## 集成步骤（5分钟）

1. 复制 `game_detector.rs` 和 `game_profile.rs` → `src-tauri/src/`
2. 用 `lib.rs` 覆盖 `src-tauri/src/lib.rs`（或手动添加 `mod` + `manage` + `invoke_handler` 3处改动）
3. 复制 `GameProfiles.tsx` → `src/pages/`
4. 复制 `GameDetectorBadge.tsx` → `src/components/`
5. 在路由表添加 `/game-profiles` 路径
6. 在顶部栏插入 `<GameDetectorBadge />` 组件
7. `npm run tauri:dev` 启动

详细步骤见 `GAME_DETECTOR_INTEGRATION.md`

## 前置要求

| 组件 | 状态 |
|------|:----:|
| Node.js 18+ | ✅ 当前环境已有 (v24.16) |
| Rust 1.70+ | ❌ 需要安装（`winget install Rustup` 或去 rustup.rs） |
| VS Build Tools (C++) | ❌ 需要安装（用于 MSVC 链接） |

### 快速装 Rust
```bash
winget install Rustup
rustup default stable
```

### 快速装 VS Build Tools
下载 [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
安装时勾选 "Desktop development with C++" 即可（约 500MB 下载）。
