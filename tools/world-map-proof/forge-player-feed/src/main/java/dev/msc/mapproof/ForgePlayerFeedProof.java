package dev.msc.mapproof;

import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;
import net.minecraftforge.event.TickEvent;
import net.minecraftforge.fml.common.Mod;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Mod("msc_map_player_feed_proof")
public final class ForgePlayerFeedProof {
    private static final String PREFIX = "MSC_MAP_PLAYERS_V1 ";
    private static final Logger LOGGER = LoggerFactory.getLogger("MSC Map Player Feed Proof");
    private long sequence;
    private int ticks;
    private boolean errorReported;

    public ForgePlayerFeedProof() {
        TickEvent.ServerTickEvent.Post.BUS.addListener(this::onServerTick);
    }

    private void onServerTick(TickEvent.ServerTickEvent.Post event) {
        if (++ticks < 20) {
            return;
        }
        ticks = 0;
        try {
            MinecraftServer server = event.server();
            StringBuilder json = new StringBuilder(256);
            json.append("{\"sequence\":").append(++sequence)
                    .append(",\"tick\":").append(server.getTickCount())
                    .append(",\"sampledAtMs\":").append(System.currentTimeMillis())
                    .append(",\"players\":[");
            boolean first = true;
            for (ServerPlayer player : server.getPlayerList().getPlayers()) {
                if (!first) {
                    json.append(',');
                }
                first = false;
                json.append("{\"id\":");
                appendString(json, player.getUUID().toString());
                json.append(",\"name\":");
                appendString(json, player.getGameProfile().name());
                json.append(",\"dimension\":");
                appendString(json, player.level().dimension().identifier().toString());
                json.append(",\"x\":").append(player.getX())
                        .append(",\"y\":").append(player.getY())
                        .append(",\"z\":").append(player.getZ())
                        .append(",\"yaw\":").append(player.getYRot())
                        .append(",\"pitch\":").append(player.getXRot())
                        .append('}');
            }
            json.append("]}");
            LOGGER.info(PREFIX + json);
            errorReported = false;
        } catch (Exception error) {
            if (!errorReported) {
                errorReported = true;
                LOGGER.error("MSC_MAP_PLAYERS_ERROR", error);
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
