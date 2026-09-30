package dev.msc.mapproof;

import org.bukkit.Bukkit;
import org.bukkit.Location;
import org.bukkit.entity.Player;
import org.bukkit.plugin.java.JavaPlugin;

public final class PaperPlayerFeedProof extends JavaPlugin {
    private static final String PREFIX = "MSC_MAP_PLAYERS_V1 ";
    private long sequence = 0;
    private boolean errorReported = false;

    @Override
    public void onEnable() {
        // Player and world reads stay on Paper's main thread.
        Bukkit.getScheduler().runTaskTimer(this, this::samplePlayers, 20L, 20L);
    }

    private void samplePlayers() {
        try {
            StringBuilder json = new StringBuilder(256);
            json.append("{\"sequence\":").append(++sequence)
                    .append(",\"tick\":").append(Bukkit.getCurrentTick())
                    .append(",\"sampledAtMs\":").append(System.currentTimeMillis())
                    .append(",\"players\":[");
            boolean first = true;
            for (Player player : Bukkit.getOnlinePlayers()) {
                Location location = player.getLocation();
                if (!first) {
                    json.append(',');
                }
                first = false;
                json.append("{\"id\":");
                appendString(json, player.getUniqueId().toString());
                json.append(",\"name\":");
                appendString(json, player.getName());
                json.append(",\"dimension\":");
                appendString(json, location.getWorld().getKey().toString());
                json.append(",\"x\":").append(location.getX())
                        .append(",\"y\":").append(location.getY())
                        .append(",\"z\":").append(location.getZ())
                        .append(",\"yaw\":").append(location.getYaw())
                        .append(",\"pitch\":").append(location.getPitch())
                        .append('}');
            }
            json.append("]}");
            getLogger().info(PREFIX + json);
            errorReported = false;
        } catch (Exception error) {
            if (!errorReported) {
                errorReported = true;
                getLogger().severe("MSC_MAP_PLAYERS_ERROR " + error);
            }
        }
    }

    private static void appendString(StringBuilder json, String value) {
        json.append('"');
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            switch (character) {
                case '"' -> json.append("\\\"");
                case '\\' -> json.append("\\\\");
                case '\n' -> json.append("\\n");
                case '\r' -> json.append("\\r");
                case '\t' -> json.append("\\t");
                default -> {
                    if (character < 0x20) {
                        json.append(String.format("\\u%04x", (int) character));
                    } else {
                        json.append(character);
                    }
                }
            }
        }
        json.append('"');
    }
}
