const PREFIX = "MSC_MAP_PLAYERS_V1 ";
let tick = 0;
let sequence = 0;
var errorReported = false;

ServerEvents.tick((event) => {
  tick += 1;
  if (tick % 20 !== 0) return;

  try {
    var onlinePlayers = event.server.getPlayerList().getPlayers();
    var players = [];
    for (var index = 0; index < onlinePlayers.size(); index += 1) {
      var player = onlinePlayers.get(index);
      players.push({
        id: player.getStringUuid(),
        name: player.getUsername(),
        dimension: player.getLevel().getDimension().toString(),
        x: player.getX(),
        y: player.getY(),
        z: player.getZ(),
        yaw: player.getYaw(),
        pitch: player.getPitch(),
      });
    }

    console.info(PREFIX + JSON.stringify({
      sequence: ++sequence,
      tick: tick,
      sampledAtMs: Date.now(),
      players: players,
    }));
    errorReported = false;
  } catch (error) {
    if (!errorReported) {
      errorReported = true;
      console.error("MSC_MAP_PLAYERS_ERROR " + String(error));
    }
  }
});
