import { system, world } from "@minecraft/server";

const PREFIX = "MSC_MAP_PLAYERS_V1 ";
let sequence = 0;

// Emit one complete roster per second so an empty roster or a stopped feed
// cannot be mistaken for a player standing still.
system.runInterval(() => {
  try {
    const players = world.getAllPlayers().map((player) => {
      const location = player.location;
      const rotation = player.getRotation();
      return {
        id: player.id,
        name: player.name,
        dimension: player.dimension.id,
        x: location.x,
        y: location.y,
        z: location.z,
        pitch: rotation.x,
        yaw: rotation.y,
      };
    });
    console.warn(PREFIX + JSON.stringify({
      sequence: ++sequence,
      tick: system.currentTick,
      sampledAtMs: Date.now(),
      players,
    }));
  } catch (error) {
    console.warn("MSC_MAP_PLAYERS_ERROR " + String(error));
  }
}, 20);
