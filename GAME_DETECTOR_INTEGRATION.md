# NexBox 升级指南：自动游戏检测引擎

## 一、Rust 后端集成

### 1.1 添加依赖

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中补充：

```toml
lazy_static = "1.5"  # 用于构建静态游戏数据库 HashMap
```

`windows` crate 已经配置了 `Win32_UI_WindowsAndMessaging` 和 `Win32_System_Threading`，不需要额外 feature。

### 1.2 添加新文件

将以下两个文件复制到 `src-tauri/src/`：

| 文件 | 复制到 |
|------|--------|
| `game_detector.rs` | `src-tauri/src/game_detector.rs` |
| `game_profile.rs` | `src-tauri/src/game_profile.rs` |

### 1.3 注册模块和命令

修改 `src-tauri/src/lib.rs`：

```rust
// 在文件顶部 mod 声明区添加：
mod game_detector;
mod game_profile;

// 在 run() 函数的 .setup() 回调中，在匹配位置添加：
.use(game_profile::ProfileState::new())
.manage(game_detector::GameDetectorState::default())

// 在 setup 闭包末尾，return Ok(()) 之前添加：
// 启动游戏检测轮询
{
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        game_detector::start_detector_loop(app_handle).await;
    });
}

// 在 .invoke_handler(tauri::generate_handler![...]) 中添加：
// game_detector commands
game_detector::get_current_game,
game_detector::set_game_detector_enabled,
game_detector::get_game_detector_enabled,
game_detector::get_known_games,
game_detector::get_game_categories,
// game_profile commands
game_profile::get_game_profiles,
game_profile::get_game_profile,
game_profile::save_game_profile,
game_profile::delete_game_profile,
game_profile::apply_preset_profile,
game_profile::get_preset_profiles,
game_profile::auto_apply_profile,
```

## 二、前端集成

### 2.1 新增页面

在 `src/pages/` 下新建 `GameProfiles.tsx`（完整代码见后文）。在路由中添加：

```tsx
// src/App.tsx 或 src/router.tsx 中添加路由
<Route path="/game-profiles" element={<GameProfiles />} />
```

在导航菜单添加入口：

```tsx
// 在侧边栏/导航栏添加
{ icon: <Gamepad2 />, label: "游戏配置", path: "/game-profiles" }
```

### 2.2 状态栏显示当前游戏

在 `src/components/` 下新建 `GameDetectorBadge.tsx`，在顶部栏显示当前检测到的游戏 + 快速开关。

### 2.3 修改 delta_force.rs（可选增强）

在原有 `delta_force.rs` 中添加一条 Tauri 命令，监听游戏检测事件并自动获取密码：

```rust
use crate::game_detector::DetectedGame;

#[tauri::command]
pub fn on_game_started(app: AppHandle, game: DetectedGame) {
    if game.process_name.contains("DeltaForce") {
        log::info!("[DeltaForce] Game started, auto-fetching password...");
        let _ = app.emit("delta-force-ready", true);
    }
}
```

## 三、完整前端组件

### 游戏配置管理页面

```tsx
// src/pages/GameProfiles.tsx
import { useEffect, useState } from "react";
import {
  Box, VStack, HStack, Text, Heading, Switch, Button,
  Select, Card, CardBody, Tag, TagLabel, SimpleGrid,
  useToast, Spinner, Badge, Divider, IconButton,
} from "@chakra-ui/react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { Gamepad2, Plus, Trash2, Play, RotateCcw } from "lucide-react";

interface DetectedGame {
  process_name: string;
  display_name: string;
  category: string;
  icon: string;
  pid: number;
}

interface GameProfile {
  process_name: string;
  display_name: string;
  icon: string;
  category: string;
  optimizations: GameOptimizations;
  auto_optimize: boolean;
}

interface GameOptimizations {
  clean_memory: boolean;
  trim_working_set: boolean;
  clean_standby: boolean;
  high_performance_power: boolean;
  kill_wallpaper_engine: boolean;
  set_game_high_priority: boolean;
  close_background_apps: boolean;
  flush_dns: boolean;
  disable_nagle: boolean;
  set_gaming_tcp: boolean;
  disable_game_bar: boolean;
  disable_notifications: boolean;
  disable_animations: boolean;
}

interface PresetInfo {
  index: number;
  name: string;
  description: string;
}

const OPTION_LABELS: Record<keyof GameOptimizations, string> = {
  clean_memory: "清理内存",
  trim_working_set: "缩减工作集",
  clean_standby: "清理备用内存",
  high_performance_power: "高性能电源计划",
  kill_wallpaper_engine: "关闭 Wallpaper Engine",
  set_game_high_priority: "游戏进程高优先级",
  close_background_apps: "关闭后台应用",
  flush_dns: "刷新 DNS",
  disable_nagle: "禁用 Nagle 算法",
  set_gaming_tcp: "游戏 TCP 优化 (BBR2)",
  disable_game_bar: "禁用游戏栏 (Game Bar)",
  disable_notifications: "禁用通知 (免打扰)",
  disable_animations: "禁用动画特效",
};

const CATEGORY_COLORS: Record<string, string> = {
  FPS: "red",
  MOBA: "orange",
  RPG: "purple",
  Strategy: "blue",
  Simulation: "green",
  Other: "gray",
};

export default function GameProfiles() {
  const [currentGame, setCurrentGame] = useState<DetectedGame | null>(null);
  const [detectorEnabled, setDetectorEnabled] = useState(true);
  const [profiles, setProfiles] = useState<GameProfile[]>([]);
  const [knownGames, setKnownGames] = useState<any[]>([]);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedGame, setSelectedGame] = useState("");
  const [selectedPreset, setSelectedPreset] = useState(0);
  const toast = useToast();

  useEffect(() => {
    const setup = async () => {
      setKnownGames(await invoke("get_known_games"));
      setPresets(await invoke("get_preset_profiles"));
      setProfiles(await invoke("get_game_profiles"));
      setLoading(false);
    };
    setup();

    // Listen for game detection events
    const unlistenPromise = listen<{ game: DetectedGame | null; action: string }>(
      "game-detector",
      (event) => {
        setCurrentGame(event.payload.game);
        if (event.payload.action === "started" && event.payload.game) {
          toast({
            title: `🕹️ 检测到 ${event.payload.game.display_name}`,
            status: "info",
            duration: 3000,
          });
        }
      }
    );

    // Refresh profiles periodically
    const interval = setInterval(async () => {
      setProfiles(await invoke("get_game_profiles"));
    }, 5000);

    return () => {
      unlistenPromise.then((fn) => fn());
      clearInterval(interval);
    };
  }, []);

  const toggleDetector = async () => {
    const next = !detectorEnabled;
    await invoke("set_game_detector_enabled", { enabled: next });
    setDetectorEnabled(next);
  };

  const addProfile = async () => {
    if (!selectedGame) return;
    const game = knownGames.find((g) => g.process_name === selectedGame);
    if (!game) return;
    await invoke("apply_preset_profile", {
      processName: game.process_name,
      displayName: game.display_name,
      presetIndex: selectedPreset,
    });
    setProfiles(await invoke("get_game_profiles"));
    toast({ title: `已添加 ${game.display_name}`, status: "success" });
  };

  const deleteProfile = async (processName: string) => {
    await invoke("delete_game_profile", { processName });
    setProfiles(await invoke("get_game_profiles"));
  };

  const toggleAutoOpt = async (profile: GameProfile) => {
    const updated = { ...profile, auto_optimize: !profile.auto_optimize };
    await invoke("save_game_profile", { profile: updated });
    setProfiles(await invoke("get_game_profiles"));
  };

  if (loading) return <Center h="400px"><Spinner /></Center>;

  return (
    <Box p={6}>
      <VStack spacing={6} align="stretch">
        {/* Header */}
        <HStack justify="space-between">
          <HStack>
            <Gamepad2 size={24} />
            <Heading size="lg">游戏配置</Heading>
          </HStack>
          <HStack>
            <Text fontSize="sm">自动检测</Text>
            <Switch isChecked={detectorEnabled} onChange={toggleDetector} />
          </HStack>
        </HStack>

        {/* Current Game Status */}
        <Card bg="whiteAlpha.50">
          <CardBody>
            <HStack justify="space-between">
              <HStack>
                <Text fontWeight="bold">当前正在运行：</Text>
                {currentGame ? (
                  <HStack>
                    <Badge colorScheme={CATEGORY_COLORS[currentGame.category] || "gray"}>
                      {currentGame.icon} {currentGame.category}
                    </Badge>
                    <Text>{currentGame.display_name}</Text>
                    <Text fontSize="xs" color="gray.500">PID: {currentGame.pid}</Text>
                  </HStack>
                ) : (
                  <Text color="gray.500">无</Text>
                )}
              </HStack>
              {currentGame && (
                <Button size="sm" colorScheme="blue" onClick={async () => {
                  const result = await invoke("auto_apply_profile", { game: currentGame });
                  toast({ title: "优化已应用", description: result, status: "success" });
                }}>
                  应用优化
                </Button>
              )}
            </HStack>
          </CardBody>
        </Card>

        {/* Add Profile */}
        <Card>
          <CardBody>
            <VStack align="stretch" spacing={3}>
              <Heading size="sm">添加游戏配置</Heading>
              <HStack>
                <Select
                  placeholder="选择游戏..."
                  value={selectedGame}
                  onChange={(e) => setSelectedGame(e.target.value)}
                >
                  {knownGames
                    .filter((g) => !profiles.find((p) => p.process_name === g.process_name))
                    .map((g) => (
                      <option key={g.process_name} value={g.process_name}>
                        {g.icon} {g.display_name}
                      </option>
                    ))}
                </Select>
                <Select
                  value={selectedPreset}
                  onChange={(e) => setSelectedPreset(Number(e.target.value))}
                  w="200px"
                >
                  {presets.map((p) => (
                    <option key={p.index} value={p.index}>
                      {p.name} — {p.description}
                    </option>
                  ))}
                </Select>
                <Button leftIcon={<Plus size={16} />} onClick={addProfile} isDisabled={!selectedGame}>
                  添加
                </Button>
              </HStack>
            </VStack>
          </CardBody>
        </Card>

        {/* Profile List */}
        <SimpleGrid columns={{ base: 1, md: 2 }} spacing={4}>
          {profiles.map((profile) => (
            <Card key={profile.process_name} variant="outline">
              <CardBody>
                <VStack align="stretch" spacing={3}>
                  <HStack justify="space-between">
                    <HStack>
                      <Badge colorScheme={CATEGORY_COLORS[profile.category] || "gray"}>
                        {profile.category}
                      </Badge>
                      <Text fontWeight="bold">{profile.display_name}</Text>
                      <Text fontSize="xs" color="gray.500">({profile.process_name})</Text>
                    </HStack>
                    <HStack>
                      <Switch
                        isChecked={profile.auto_optimize}
                        onChange={() => toggleAutoOpt(profile)}
                      />
                      <IconButton
                        aria-label="Delete"
                        icon={<Trash2 size={14} />}
                        size="xs"
                        variant="ghost"
                        colorScheme="red"
                        onClick={() => deleteProfile(profile.process_name)}
                      />
                    </HStack>
                  </HStack>

                  <Text fontSize="xs" color="gray.400">
                    自动优化：{profile.auto_optimize ? "开启" : "关闭"}
                  </Text>

                  <Divider />

                  <SimpleGrid columns={2} spacing={1}>
                    {(Object.keys(profile.optimizations) as (keyof GameOptimizations)[]).map((key) => (
                      <HStack key={key} spacing={2}>
                        <Box
                          w={2} h={2} borderRadius="full"
                          bg={profile.optimizations[key] ? "green.400" : "gray.600"}
                        />
                        <Text fontSize="xs">{OPTION_LABELS[key]}</Text>
                      </HStack>
                    ))}
                  </SimpleGrid>
                </VStack>
              </CardBody>
            </Card>
          ))}
        </SimpleGrid>

        {profiles.length === 0 && (
          <Text color="gray.500" textAlign="center" py={8}>
            还没有添加任何游戏配置。选择一个游戏和预设方案，点击"添加"开始。
          </Text>
        )}
      </VStack>
    </Box>
  );
}

// Need Center import
import { Center } from "@chakra-ui/react";
```

### 检测状态徽标组件

```tsx
// src/components/GameDetectorBadge.tsx
import { useEffect, useState } from "react";
import { HStack, Badge, Switch, Text, Tooltip } from "@chakra-ui/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DetectedGame {
  display_name: string;
  icon: string;
}

export default function GameDetectorBadge() {
  const [game, setGame] = useState<DetectedGame | null>(null);
  const [enabled, setEnabled] = useState(true);

  useEffect(() => {
    invoke("get_current_game").then(setGame);
    invoke("get_game_detector_enabled").then(setEnabled);

    const unsub = listen<{ game: DetectedGame | null }>("game-detector", (e) => {
      setGame(e.payload.game);
    });
    return () => { unsub.then((fn) => fn()); };
  }, []);

  if (!enabled) return null;

  return (
    <HStack spacing={2} px={3}>
      <Tooltip label="自动游戏检测">
        <Switch
          size="sm"
          isChecked={enabled}
          onChange={async () => {
            const next = !enabled;
            await invoke("set_game_detector_enabled", { enabled: next });
            setEnabled(next);
          }}
        />
      </Tooltip>
      {game ? (
        <Badge colorScheme="green" variant="subtle">
          {game.icon} {game.display_name}
        </Badge>
      ) : (
        <Text fontSize="xs" color="gray.500">未检测到游戏</Text>
      )}
    </HStack>
  );
}
```

## 四、集成清单

```
☐ 1. Cargo.toml 添加 lazy_static
☐ 2. 复制 game_detector.rs → src-tauri/src/
☐ 3. 复制 game_profile.rs → src-tauri/src/
☐ 4. lib.rs 添加 mod 声明 + manage + spawn + invoke_handler
☐ 5. 前端复制 GameProfiles.tsx → src/pages/
☐ 6. 前端复制 GameDetectorBadge.tsx → src/components/
☐ 7. 路由注册 /game-profiles
☐ 8. 顶部栏插入 <GameDetectorBadge />
☐ 9. 验证：npm run tauri:dev 启动，切换窗口看终端日志
```

## 五、测试方法

1. 启动 NexBox（`npm run tauri:dev`）
2. 打开任意已收录的游戏（CS2、原神、三角洲行动等）
3. 顶部栏应显示游戏名称 + 图标
4. 终端日志输出 `[GameDetector] Game started: xxx`
5. 打开「游戏配置」页面，为游戏添加预设
6. 切回游戏，应自动应用优化
7. 最小化游戏，应触发 `stopped` 事件
