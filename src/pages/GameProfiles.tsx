import { useEffect, useState } from "react";
import {
  Box, VStack, HStack, Text, Heading, Switch, Button,
  Select, Card, CardBody, Tag, TagLabel, SimpleGrid,
  useToast, Spinner, Badge, Divider, IconButton, Center,
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

interface GameProfile {
  process_name: string;
  display_name: string;
  icon: string;
  category: string;
  optimizations: GameOptimizations;
  auto_optimize: boolean;
}

interface PresetInfo {
  index: number;
  name: string;
  description: string;
}

const OPTION_LABELS: Record<string, string> = {
  clean_memory: "清理内存",
  trim_working_set: "缩减工作集",
  clean_standby: "清理备用内存",
  high_performance_power: "高性能电源计划",
  kill_wallpaper_engine: "关闭 Wallpaper Engine",
  set_game_high_priority: "游戏进程高优先级",
  close_background_apps: "关闭后台应用",
  flush_dns: "刷新 DNS",
  disable_nagle: "禁用 Nagle 算法",
  set_gaming_tcp: "游戏 TCP 优化",
  disable_game_bar: "禁用游戏栏",
  disable_notifications: "禁用通知",
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
      try {
        setKnownGames(await invoke("get_known_games"));
        setPresets(await invoke("get_preset_profiles"));
        setProfiles(await invoke("get_game_profiles"));
        const cur = await invoke<DetectedGame | null>("get_current_game");
        setCurrentGame(cur);
        const en = await invoke<boolean>("get_game_detector_enabled");
        setDetectorEnabled(en);
      } catch (e) {
        console.error("Failed to load game detector data:", e);
      }
      setLoading(false);
    };
    setup();

    const unlistenPromise = listen<{ game: DetectedGame | null; action: string }>(
      "game-detector",
      (event) => {
        setCurrentGame(event.payload.game);
        if (event.payload.action === "started" && event.payload.game) {
          toast({
            title: `🕹️ ${event.payload.game.display_name}`,
            description: "已自动检测到游戏运行",
            status: "info",
            duration: 3000,
            variant: "subtle",
          });
          // Triggger auto-apply
          invoke("auto_apply_profile", { game: event.payload.game }).catch(() => {});
        }
      }
    );

    const interval = setInterval(async () => {
      try {
        setProfiles(await invoke("get_game_profiles"));
      } catch {}
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
    try {
      await invoke("apply_preset_profile", {
        processName: game.process_name,
        displayName: game.display_name,
        presetIndex: selectedPreset,
      });
      setProfiles(await invoke("get_game_profiles"));
      toast({ title: `已添加 ${game.display_name}`, status: "success" });
      setSelectedGame("");
    } catch (e) {
      toast({ title: "添加失败", description: String(e), status: "error" });
    }
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

  const applyNow = async () => {
    if (!currentGame) return;
    try {
      const result = await invoke<string>("auto_apply_profile", { game: currentGame });
      toast({ title: "✅ 优化已应用", description: result, status: "success" });
    } catch (e) {
      toast({ title: "优化失败", description: String(e), status: "error" });
    }
  };

  if (loading) return <Center h="400px"><Spinner size="xl" /></Center>;

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
            <Text fontSize="sm" color="gray.400">自动检测</Text>
            <Switch isChecked={detectorEnabled} onChange={toggleDetector} />
          </HStack>
        </HStack>

        {/* Current Game Status */}
        <Card bg="whiteAlpha.50" borderColor={currentGame ? "green.500" : "gray.600"}>
          <CardBody>
            <HStack justify="space-between">
              <HStack spacing={3}>
                <Text fontWeight="bold" fontSize="sm" color="gray.400">当前运行：</Text>
                {currentGame ? (
                  <HStack>
                    <Badge colorScheme={CATEGORY_COLORS[currentGame.category] || "gray"} fontSize="sm" px={2} py={1}>
                      {currentGame.icon} {currentGame.category}
                    </Badge>
                    <Text fontWeight="bold" fontSize="lg">{currentGame.display_name}</Text>
                    <Text fontSize="xs" color="gray.500" fontFamily="mono">PID {currentGame.pid}</Text>
                  </HStack>
                ) : (
                  <Text color="gray.500" fontStyle="italic">无游戏运行</Text>
                )}
              </HStack>
              {currentGame && (
                <Button
                  size="sm"
                  colorScheme="green"
                  leftIcon={<Play size={14} />}
                  onClick={applyNow}
                >
                  立即优化
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
                  w="220px"
                >
                  {presets.map((p, i) => (
                    <option key={i} value={i}>
                      {p.name}
                    </option>
                  ))}
                </Select>
                <Button leftIcon={<Plus size={16} />} onClick={addProfile} isDisabled={!selectedGame}>
                  添加
                </Button>
              </HStack>
              {selectedPreset < presets.length && (
                <Text fontSize="xs" color="gray.500">
                  {presets[selectedPreset].description}
                </Text>
              )}
            </VStack>
          </CardBody>
        </Card>

        {/* Profile List */}
        {profiles.length === 0 ? (
          <Text color="gray.500" textAlign="center" py={8}>
            还没有游戏配置。选一个游戏和预设方案，点击添加。
          </Text>
        ) : (
          <SimpleGrid columns={{ base: 1, md: 2 }} spacing={4}>
            {profiles.map((profile) => (
              <Card key={profile.process_name} variant="outline">
                <CardBody>
                  <VStack align="stretch" spacing={3}>
                    {/* Header row */}
                    <HStack justify="space-between">
                      <HStack>
                        <Badge colorScheme={CATEGORY_COLORS[profile.category] || "gray"}>
                          {profile.category}
                        </Badge>
                        <Text fontWeight="bold">{profile.display_name}</Text>
                      </HStack>
                      <HStack spacing={2}>
                        <Switch
                          size="sm"
                          isChecked={profile.auto_optimize}
                          onChange={() => toggleAutoOpt(profile)}
                        />
                        <IconButton
                          aria-label="删除"
                          icon={<Trash2 size={14} />}
                          size="xs"
                          variant="ghost"
                          colorScheme="red"
                          onClick={() => deleteProfile(profile.process_name)}
                        />
                      </HStack>
                    </HStack>

                    <HStack>
                      <Tag
                        size="sm"
                        colorScheme={profile.auto_optimize ? "green" : "gray"}
                      >
                        <TagLabel>
                          {profile.auto_optimize ? "自动优化已开启" : "自动优化已关闭"}
                        </TagLabel>
                      </Tag>
                    </HStack>

                    <Divider />

                    {/* Optimization list */}
                    <SimpleGrid columns={2} spacing={1}>
                      {(Object.keys(profile.optimizations) as (keyof GameOptimizations)[]).map((key) => (
                        <HStack key={key} spacing={2}>
                          <Box
                            w="6px" h="6px" borderRadius="full"
                            bg={profile.optimizations[key] ? "green.400" : "gray.600"}
                            flexShrink={0}
                          />
                          <Text fontSize="xs" color={profile.optimizations[key] ? "gray.200" : "gray.500"}>
                            {OPTION_LABELS[key] || key}
                          </Text>
                        </HStack>
                      ))}
                    </SimpleGrid>
                  </VStack>
                </CardBody>
              </Card>
            ))}
          </SimpleGrid>
        )}
      </VStack>
    </Box>
  );
}
