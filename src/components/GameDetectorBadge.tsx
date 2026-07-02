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
    invoke<DetectedGame | null>("get_current_game").then(setGame).catch(() => {});
    invoke<boolean>("get_game_detector_enabled").then(setEnabled).catch(() => {});

    const unsub = listen<{ game: DetectedGame | null }>("game-detector", (e) => {
      setGame(e.payload.game);
    });
    return () => { unsub.then((fn) => fn()); };
  }, []);

  if (!enabled) return null;

  return (
    <HStack spacing={2} px={3}>
      <Tooltip label="自动游戏检测已开启" placement="bottom">
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
        <Badge colorScheme="green" variant="subtle" fontSize="xs" px={2} py={0.5}>
          {game.icon} {game.display_name}
        </Badge>
      ) : (
        <Text fontSize="xs" color="gray.500">待机</Text>
      )}
    </HStack>
  );
}
