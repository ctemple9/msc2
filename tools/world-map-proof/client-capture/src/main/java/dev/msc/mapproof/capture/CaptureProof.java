package dev.msc.mapproof.capture;

import com.google.gson.*;
import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.*;
import com.mojang.brigadier.Command;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.renderer.*;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.core.*;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.*;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.RandomSource;
import net.minecraft.world.Container;
import net.minecraft.world.item.*;
import net.minecraft.world.level.BlockAndTintGetter;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.RenderShape;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.storage.LevelResource;
import net.neoforged.api.distmarker.Dist;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.client.event.RegisterClientCommandsEvent;
import net.neoforged.neoforge.client.event.RenderLevelStageEvent;
import net.neoforged.neoforge.common.NeoForge;
import org.lwjgl.opengl.GL11;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.time.Instant;
import java.util.*;
import java.util.stream.Stream;
import static net.minecraft.commands.Commands.literal;

@Mod(value = "msc_mesh_capture_proof", dist = Dist.CLIENT)
public final class CaptureProof {
    static final Gson JSON = new GsonBuilder().setPrettyPrinting().create();
    static final Path WORK = Path.of(System.getProperty("msc.proof.workspace"));
    static final BlockPos LEFT = new BlockPos(3,65,3), RIGHT = new BlockPos(6,65,3), CUSTOM = new BlockPos(9,65,3);
    static final BlockPos LOW = new BlockPos(2,64,2), HIGH = new BlockPos(10,67,5);
    static volatile Binding binding;
    static boolean captureRequested;
    record Binding(String snapshot, String context, String resources, long gameTick, Path world) {}

    public CaptureProof(IEventBus bus) {
        NeoForge.EVENT_BUS.addListener(this::commands);
        NeoForge.EVENT_BUS.addListener(this::frame);
    }

    void commands(RegisterClientCommandsEvent event) {
        event.getDispatcher().register(literal("mscproof")
            .then(literal("setup").executes(c -> setup()))
            .then(literal("capture").executes(c -> prepareCapture())));
    }

    static void tell(String message) {
        Minecraft mc = Minecraft.getInstance();
        mc.execute(() -> { if (mc.player != null) mc.player.displayClientMessage(Component.literal("MSC proof: " + message), false); });
    }

    static MinecraftServer isolatedServer() throws IOException {
        Minecraft mc = Minecraft.getInstance();
        Path client = WORK.resolve("client").toRealPath();
        if (!mc.gameDirectory.toPath().toRealPath().equals(client) || !Files.isRegularFile(client.resolve(".msc-proof-client")))
            throw new IOException("Refused: this is not the prepared isolated proof client.");
        MinecraftServer server = mc.getSingleplayerServer();
        if (server == null || !server.getWorldData().getLevelName().equals("MSC Mesh Capture Proof"))
            throw new IOException("Refused: open a new singleplayer world named MSC Mesh Capture Proof in this isolated client.");
        Path world = server.getWorldPath(LevelResource.ROOT).toRealPath();
        if (!world.startsWith(client.resolve("saves"))) throw new IOException("Refused: world is outside the isolated proof saves.");
        return server;
    }

    int setup() {
        try {
            MinecraftServer server = isolatedServer();
            server.execute(() -> {
                try {
                    ServerLevel level = server.overworld();
                    if (!level.hasChunk(0,0)) throw new IOException("Refused: fixture chunk 0,0 is not loaded. Use /tp @s 6 67 11 and wait before setup.");
                    // Only this new, explicitly prepared proof world may be modified.
                    for (BlockPos p : BlockPos.betweenClosed(LOW, HIGH)) level.setBlock(p, p.getY()==64 ? Blocks.STONE.defaultBlockState() : Blocks.AIR.defaultBlockState(), 3);
                    Block pedestal = BuiltInRegistries.BLOCK.get(ResourceLocation.parse("supplementaries:pedestal"));
                    Block barnacles = BuiltInRegistries.BLOCK.get(ResourceLocation.parse("supplementaries:barnacles"));
                    if (pedestal == Blocks.AIR || barnacles == Blocks.AIR) throw new IOException("Pinned Supplementaries fixture blocks are unavailable.");
                    level.setBlock(LEFT, pedestal.defaultBlockState(), 3);
                    level.setBlock(RIGHT, pedestal.defaultBlockState(), 3);
                    if (!(level.getBlockEntity(LEFT) instanceof Container a) || !(level.getBlockEntity(RIGHT) instanceof Container b))
                        throw new IOException("Pedestal is not the pinned item-display container.");
                    a.setItem(0,new ItemStack(Items.DIAMOND)); b.setItem(0,new ItemStack(Items.EMERALD));
                    level.getBlockEntity(LEFT).setChanged(); level.getBlockEntity(RIGHT).setChanged();
                    level.sendBlockUpdated(LEFT,level.getBlockState(LEFT),level.getBlockState(LEFT),3);
                    level.sendBlockUpdated(RIGHT,level.getBlockState(RIGHT),level.getBlockState(RIGHT),3);
                    level.setBlock(CUSTOM.north(),Blocks.STONE.defaultBlockState(),3);
                    BlockState customState = barnacles.defaultBlockState();
                    var north = customState.getProperties().stream().filter(p -> p.getName().equals("north")).findFirst().orElseThrow();
                    customState = setTrue(customState,north);
                    level.setBlock(CUSTOM,customState,3);
                    level.setDayTime(6000);
                    server.tickRateManager().setFrozen(true);
                    server.saveEverything(true,true,true);
                    Path world = server.getWorldPath(LevelResource.ROOT).toRealPath();
                    binding = new Binding(snapshot(world),context(level),resources(),level.getGameTime(),world);
                    Minecraft.getInstance().execute(() -> { if (Minecraft.getInstance().player != null) Minecraft.getInstance().player.connection.sendCommand("tp @s 6 67 11 180 20"); });
                    tell("Fixture saved and simulation frozen. Diamond pedestal (3,65,3), emerald pedestal (6,65,3), custom-loader barnacles (9,65,3). Wait until visible, then /mscproof capture. A saved-frame export will also check refusals.");
                } catch (Exception e) { tell(e.getMessage()); }
            });
        } catch (Exception e) { tell(e.getMessage()); }
        return Command.SINGLE_SUCCESS;
    }

    int prepareCapture() {
        try {
            MinecraftServer server = isolatedServer();
            Binding fixture = binding;
            if (fixture == null) throw new IOException("needs_setup: /mscproof setup first");
            Minecraft mc = Minecraft.getInstance();
            if (mc.level == null || !mc.level.dimension().equals(Level.OVERWORLD)) throw new IOException("dimension_mismatch");
            if (!fixture.context.equals(context(mc.level))) throw new IOException("context_mismatch: wait for updates or rerun setup if the fixture changed");
            server.execute(() -> {
                try {
                    if (binding != fixture) throw new IOException("setup_changed_during_capture_request");
                    if (!server.tickRateManager().isFrozen()) throw new IOException("simulation_not_frozen: run setup again");
                    if (!fixture.context.equals(context(server.overworld()))) throw new IOException("context_mismatch: saved fixture contents changed");
                    if (!fixture.resources.equals(resources())) throw new IOException("resource_context_mismatch");
                    // Integrated-server pause/autosaves can rewrite metadata while simulation is frozen.
                    // An explicit capture request binds a freshly flushed save, after checking setup context.
                    server.saveEverything(true,true,true);
                    Binding capture = new Binding(snapshot(fixture.world), fixture.context, fixture.resources,
                            server.overworld().getGameTime(), fixture.world);
                    mc.execute(() -> {
                        if (binding != fixture) { tell("Capture refused: setup_changed_during_capture_request"); return; }
                        binding = capture;
                        captureRequested = true;
                    });
                } catch (Exception e) { tell("Capture refused: " + e.getMessage()); }
            });
        } catch (Exception e) { tell("Capture refused: " + e.getMessage()); }
        return Command.SINGLE_SUCCESS;
    }

    @SuppressWarnings({"unchecked","rawtypes"})
    static BlockState setTrue(BlockState state, net.minecraft.world.level.block.state.properties.Property property) {
        return state.setValue(property,Boolean.TRUE);
    }

    void frame(RenderLevelStageEvent event) {
        if (event.getStage()!=RenderLevelStageEvent.Stage.AFTER_LEVEL || !captureRequested) return;
        captureRequested=false;
        try { exportFrame(event); tell("Capture exported. Open the proof viewer; visual acceptance remains pending."); }
        catch (Exception e) { tell("Capture refused: " + e.getMessage()); }
    }

    // Canonicalize update data instead of relying on NBT map iteration order.
    static String canonical(Tag tag) {
        if (tag instanceof CompoundTag c) {
            StringBuilder b = new StringBuilder("{");
            for (String key : new TreeSet<>(c.getAllKeys())) b.append(JSON.toJson(key)).append(':').append(canonical(c.get(key))).append(';');
            return b.append('}').toString();
        }
        if (tag instanceof ListTag l) {
            StringBuilder b = new StringBuilder("["); for (Tag child:l) b.append(canonical(child)).append(';'); return b.append(']').toString();
        }
        return tag.toString();
    }

    static String state(BlockState state) {
        List<String> props = new ArrayList<>();
        state.getValues().forEach((p,v) -> props.add(p.getName()+"="+value(p,v)));
        Collections.sort(props);
        return BuiltInRegistries.BLOCK.getKey(state.getBlock())+"["+String.join(",",props)+"]";
    }
    @SuppressWarnings({"unchecked","rawtypes"}) static String value(net.minecraft.world.level.block.state.properties.Property p, Comparable v) { return p.getName(v); }

    static String context(Level level) throws Exception {
        StringBuilder b = new StringBuilder(level.dimension().location().toString());
        for (BlockPos p : BlockPos.betweenClosed(LOW.offset(-1,-1,-1),HIGH.offset(1,1,1))) {
            if (!level.hasChunk(p.getX()>>4,p.getZ()>>4)) throw new IOException("unloaded_chunk: " + (p.getX()>>4)+","+(p.getZ()>>4));
            b.append(p.toShortString()).append(state(level.getBlockState(p)));
            BlockEntity entity=level.getBlockEntity(p);
            if (entity!=null) b.append(canonical(entity.getUpdateTag(level.registryAccess())));
        }
        return sha(b.toString().getBytes(StandardCharsets.UTF_8));
    }

    static String snapshot(Path world) throws Exception {
        List<Path> paths=new ArrayList<>(); paths.add(world.resolve("level.dat"));
        try (Stream<Path> stream=Files.list(world.resolve("region"))) { paths.addAll(stream.filter(p->p.getFileName().toString().endsWith(".mca")).sorted().toList()); }
        if (paths.size()>128) throw new IOException("snapshot_file_limit");
        long total=0; StringBuilder receipt=new StringBuilder();
        for (Path p:paths) {
            if (Files.isSymbolicLink(p)||!Files.isRegularFile(p)) throw new IOException("snapshot_path_refused");
            long size=Files.size(p); total+=size; if(size>32*1024*1024L||total>128*1024*1024L) throw new IOException("snapshot_byte_limit");
            byte[] bytes=Files.readAllBytes(p);
            if(bytes.length!=size) throw new IOException("snapshot_changed");
            receipt.append(world.relativize(p)).append(':').append(sha(bytes)).append('\n');
        }
        return sha(receipt.toString().getBytes(StandardCharsets.UTF_8));
    }

    static String resources() throws Exception {
        Minecraft mc=Minecraft.getInstance();
        if(!mc.options.resourcePacks.isEmpty())throw new IOException("unapproved_resource_pack_selection: use the isolated default client resources");
        StringBuilder receipt=new StringBuilder(Files.readString(WORK.resolve("inputs.json")));
        receipt.append(mc.getResourcePackRepository().getSelectedIds());
        Path configs=WORK.resolve("client/config");
        long total=0;
        if(Files.exists(configs))try(Stream<Path> stream=Files.walk(configs)) {
            List<Path> files=stream.sorted().toList();if(files.size()>1024)throw new IOException("config_count_limit");
            for(Path p:files) {
                if(Files.isSymbolicLink(p))throw new IOException("config_link_refused");
                if(Files.isRegularFile(p)) {
                    long size=Files.size(p);total+=size;if(size>4*1024*1024L||total>16*1024*1024L)throw new IOException("config_byte_limit");
                    receipt.append(configs.relativize(p)).append(':').append(sha(Files.readAllBytes(p)));
                }
            }
        }
        return sha(receipt.toString().getBytes(StandardCharsets.UTF_8));
    }

    static String sha(byte[] bytes) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes)); }

    static void require(String expectedSnapshot,String expectedContext,BlockPos pos) throws Exception {
        Minecraft mc=Minecraft.getInstance(); MinecraftServer server=isolatedServer(); Binding current=binding;
        if (current==null) throw new IOException("needs_setup: /mscproof setup first");
        if (!server.tickRateManager().isFrozen()) throw new IOException("simulation_not_frozen: run setup again");
        if (mc.level==null || !mc.level.dimension().equals(Level.OVERWORLD)) throw new IOException("dimension_mismatch");
        if (!mc.level.hasChunk(pos.getX()>>4,pos.getZ()>>4)) throw new IOException("unloaded_chunk");
        if (!current.snapshot.equals(expectedSnapshot)||!snapshot(current.world).equals(expectedSnapshot)) throw new IOException("snapshot_mismatch");
        if (!current.resources.equals(resources())) throw new IOException("resource_context_mismatch");
        if (!current.context.equals(expectedContext)||!context(mc.level).equals(expectedContext)) throw new IOException("context_mismatch: wait for client updates, or rerun setup if the fixture changed");
    }

    static String refusal(String snapshot,String context,BlockPos pos,String code) throws Exception {
        try { require(snapshot,context,pos); }
        catch(IOException e) { if(e.getMessage().startsWith(code)) return code; throw e; }
        throw new IOException("Required refusal did not happen: " + code);
    }

    static void exportFrame(RenderLevelStageEvent event) throws Exception {
        Minecraft mc=Minecraft.getInstance(); Binding current=binding;
        if(current==null) throw new IOException("needs_setup");
        require(current.snapshot,current.context,LEFT);
        JsonObject refusals=new JsonObject();
        refusals.addProperty("unloadedChunk",refusal(current.snapshot,current.context,new BlockPos(16000000,65,16000000),"unloaded_chunk"));
        refusals.addProperty("snapshotMismatch",refusal("0".repeat(64),current.context,LEFT,"snapshot_mismatch"));
        refusals.addProperty("contextMismatch",refusal(current.snapshot,"0".repeat(64),LEFT,"context_mismatch"));
        if(!state(mc.level.getBlockState(LEFT)).equals(state(mc.level.getBlockState(RIGHT)))) throw new IOException("context_pair_states_differ");
        for(BlockPos pos:List.of(LEFT,RIGHT)) if(!(mc.level.getBlockEntity(pos) instanceof Container c)||c.isEmpty()) throw new IOException("context_item_missing");
        if(!((Container)mc.level.getBlockEntity(LEFT)).getItem(0).is(Items.DIAMOND)||!((Container)mc.level.getBlockEntity(RIGHT)).getItem(0).is(Items.EMERALD)) throw new IOException("context_items_mismatch");
        Path output=WORK.resolve("exports"); if(Files.isSymbolicLink(output))throw new IOException("export_link_refused");Files.createDirectories(output);
        Path candidate=Files.createTempDirectory(output,"candidate-");
        try {
            Collector collector=new Collector(candidate,event);
            JsonArray objects=new JsonArray();
            for(BlockPos pos:BlockPos.betweenClosed(LOW,HIGH)) {
                BlockState state=mc.level.getBlockState(pos); if(state.isAir()) continue;
                JsonObject object=new JsonObject(); object.addProperty("state",state(state)); object.add("position",JSON.toJsonTree(List.of(pos.getX(),pos.getY(),pos.getZ())));
                collector.object=objects.size();
                PoseStack pose=new PoseStack(); pose.translate(pos.getX(),pos.getY(),pos.getZ());
                var dispatcher=mc.getBlockRenderer(); var model=dispatcher.getBlockModel(state);
                BlockEntity entity=mc.level.getBlockEntity(pos);
                var data=entity==null?net.neoforged.neoforge.client.model.data.ModelData.EMPTY:entity.getModelData();
                data=model.getModelData(mc.level,pos,state,data);
                if(state.getRenderShape()==RenderShape.MODEL) {
                    for(RenderType type:model.getRenderTypes(state,RandomSource.create(state.getSeed(pos)),data))
                        dispatcher.renderBatched(state,pos,mc.level,pose,collector.getBuffer(type),true,RandomSource.create(state.getSeed(pos)),data,type);
                }
                int beforeEntity=collector.vertices;
                if(entity!=null) renderEntity(entity,pose,collector);
                object.addProperty("blockEntityVertices",collector.vertices-beforeEntity);
                object.addProperty("bakedModelClass",model.getClass().getName());
                if(pos.equals(LEFT)||pos.equals(RIGHT))object.addProperty("displayedItem",BuiltInRegistries.ITEM.getKey(((Container)entity).getItem(0).getItem()).toString());
                if(pos.equals(CUSTOM)) {
                    ResourceLocation location=ResourceLocation.parse("supplementaries:models/block/barnacles.json");
                    byte[] bytes;
                    try(var stream=mc.getResourceManager().getResourceOrThrow(location).open()) { bytes=stream.readNBytes(8*1024*1024+1); }
                    if(bytes.length>8*1024*1024)throw new IOException("model_resource_limit");
                    JsonObject resource=JsonParser.parseString(new String(bytes,StandardCharsets.UTF_8)).getAsJsonObject();
                    if(!resource.get("loader").getAsString().equals("supplementaries:random_rotation"))throw new IOException("custom_loader_resource_mismatch");
                    object.addProperty("modelResourceSha256",sha(bytes));object.addProperty("loader","supplementaries:random_rotation");
                }
                object.addProperty("role",pos.equals(LEFT)?"diamond_pedestal":pos.equals(RIGHT)?"emerald_pedestal":pos.equals(CUSTOM)?"custom_loader_barnacles":"surroundings");
                objects.add(object);
            }
            collector.finish();
            require(current.snapshot,current.context,RIGHT);
            JsonObject result=new JsonObject(); result.addProperty("format","msc-supplemental-mesh-proof-1");
            result.addProperty("minecraft","1.21.1");result.addProperty("neoforge","21.1.251");
            result.addProperty("snapshotSha256",current.snapshot);result.addProperty("contextSha256",current.context);
            result.addProperty("dimension","minecraft:overworld");result.addProperty("capturedAt",Instant.now().toString());
            result.addProperty("gameTick",mc.level.getGameTime());result.addProperty("snapshotGameTick",current.gameTick);result.addProperty("resourceContextSha256",current.resources);result.addProperty("partialTick",0);
            result.addProperty("appearance","saved_frame");result.addProperty("visualAcceptance","pending");
            result.add("refusals",refusals); result.add("objects",objects);
            result.add("directionalLights",collector.lights);result.add("materials",collector.materials); result.add("meshes",collector.meshes);
            result.add("inputs",JsonParser.parseString(Files.readString(WORK.resolve("inputs.json"))));
            result.add("buildReceipt",JsonParser.parseString(Files.readString(WORK.resolve("build-receipt.json"))));
            result.add("selectedPacks",JSON.toJsonTree(mc.getResourcePackRepository().getSelectedIds()));
            try(NativeImage image=Screenshot.takeScreenshot(mc.getMainRenderTarget())) { image.writeToFile(candidate.resolve("minecraft-frame.png")); }
            JsonObject files=new JsonObject();
            try(Stream<Path> stream=Files.list(candidate)) { for(Path file:stream.sorted().toList()) { JsonObject item=new JsonObject();item.addProperty("bytes",Files.size(file)); item.addProperty("sha256",sha(Files.readAllBytes(file)));files.add(file.getFileName().toString(),item); } }
            result.add("files",files); Files.writeString(candidate.resolve("capture.json"),JSON.toJson(result));
            // Publication is a new immutable directory; incomplete candidates are never current.
            Path published=output.resolve("capture-"+UUID.randomUUID()); Files.move(candidate,published,StandardCopyOption.ATOMIC_MOVE);
            Path pointer=output.resolve("current.tmp");Files.writeString(pointer,published.getFileName().toString());
            Files.move(pointer,output.resolve("current"),StandardCopyOption.REPLACE_EXISTING,StandardCopyOption.ATOMIC_MOVE);
        } finally { if(Files.exists(candidate)) try(Stream<Path> stream=Files.walk(candidate)) { for(Path p:stream.sorted(Comparator.reverseOrder()).toList()) Files.delete(p); } }
    }

    @SuppressWarnings({"unchecked","rawtypes"})
    static void renderEntity(BlockEntity entity,PoseStack pose,Collector collector) throws IOException {
        var dispatcher=Minecraft.getInstance().getBlockEntityRenderDispatcher();
        BlockEntityRenderer renderer=dispatcher.getRenderer(entity);
        if(renderer==null) throw new IOException("block_entity_renderer_missing");
        // Invoke the actual registered renderer with this position's client-visible contents.
        // Direct invocation avoids silently skipping an object due to the current camera's range.
        renderer.render(entity,0,pose,collector,LevelRenderer.getLightColor(entity.getLevel(),entity.getBlockPos()),OverlayTexture.NO_OVERLAY);
    }

    static final class Collector implements MultiBufferSource {
        final Path out;
        final Map<String,Sink> sinks=new LinkedHashMap<>();
        final Map<ResourceLocation,String> textures=new LinkedHashMap<>();
        final JsonArray materials=new JsonArray(),meshes=new JsonArray(),lights=new JsonArray();
        int object,vertices;long decodedImageBytes;
        Collector(Path out,RenderLevelStageEvent event) throws IOException {
            this.out=out;
            org.joml.Matrix3f inverse=new org.joml.Matrix3f(event.getPoseStack().last().normal()).invert();
            for(org.joml.Vector3f direction:com.mojang.blaze3d.systems.RenderSystem.shaderLightDirections) {
                if(direction==null)throw new IOException("directional_light_context_missing");
                org.joml.Vector3f world=inverse.transform(new org.joml.Vector3f(direction));
                lights.add(JSON.toJsonTree(List.of(world.x,world.y,world.z)));
            }
        }
        public VertexConsumer getBuffer(RenderType type) {
            try {
                if(type.mode()!=VertexFormat.Mode.QUADS&&type.mode()!=VertexFormat.Mode.TRIANGLES) throw new IOException("unsupported_topology: "+type.name);
                // Explicit allowlist for this pinned proof. No shader is silently approximated.
                String mode=switch(type.name) {
                    case "solid","entity_solid" -> "opaque";
                    case "cutout","cutout_mipped","entity_cutout","entity_cutout_no_cull","entity_cutout_no_cull_z_offset" -> "cutout";
                    case "translucent","entity_translucent","entity_translucent_cull","item_entity_translucent_cull" -> "translucent";
                    default -> throw new IOException("unsupported_material: "+type.name);
                };
                if(!(type instanceof RenderType.CompositeRenderType composite)) throw new IOException("unsupported_material_binding");
                ResourceLocation location=composite.state().textureState.cutoutTexture().orElseThrow(()->new IOException("missing_material_texture"));
                boolean standard=type==RenderType.solid()||type==RenderType.cutout()||type==RenderType.cutoutMipped()||type==RenderType.translucent()
                    ||type==RenderType.entitySolid(location)||type==RenderType.entityCutout(location)
                    ||type==RenderType.entityCutoutNoCull(location,true)||type==RenderType.entityCutoutNoCull(location,false)
                    ||type==RenderType.entityTranslucent(location,true)||type==RenderType.entityTranslucent(location,false)
                    ||type==RenderType.entityTranslucentCull(location)||type==RenderType.itemEntityTranslucentCull(location);
                if(!standard)throw new IOException("unsupported_custom_material_state: "+type.name);
                String texture=textures.get(location);
                if(texture==null) { texture=saveTexture(location);textures.put(location,texture); }
                String key=object+":"+type.toString();
                Sink sink=sinks.get(key);
                if(sink==null) {
                    JsonObject material=new JsonObject(); material.addProperty("renderType",type.name);material.addProperty("mode",mode);
                    material.addProperty("texture",texture);material.addProperty("alphaThreshold",mode.equals("cutout")?0.1:0);
                    material.addProperty("cull",!type.name.contains("no_cull")&&!type.name.equals("entity_translucent"));
                    material.addProperty("depthWrite",true);material.addProperty("emissive",false);
                    material.addProperty("directionalLighting",type.name.startsWith("entity_")||type.name.equals("item_entity_translucent_cull"));
                    material.addProperty("uvConvention","gpu_texture_rows_no_flip");material.addProperty("lighting","baked_client_lightmap_and_vertex_tint");
                    sink=new Sink(this,materials.size(),object,type.mode());materials.add(material);sinks.put(key,sink);
                }
                return sink;
            } catch(Exception e) { throw new IllegalStateException(e.getMessage(),e); }
        }
        String saveTexture(ResourceLocation location) throws Exception {
            if(textures.size()>=16) throw new IOException("texture_count_limit");
            int previous=GL11.glGetInteger(GL11.GL_TEXTURE_BINDING_2D);
            try {
                Minecraft.getInstance().getTextureManager().getTexture(location).bind();
                int w=GL11.glGetTexLevelParameteri(GL11.GL_TEXTURE_2D,0,GL11.GL_TEXTURE_WIDTH);
                int h=GL11.glGetTexLevelParameteri(GL11.GL_TEXTURE_2D,0,GL11.GL_TEXTURE_HEIGHT);
                if(w<1||h<1||w>8192||h>8192||(long)w*h*4>256*1024*1024) throw new IOException("texture_size_limit");
                decodedImageBytes+=(long)w*h*4;
                if(decodedImageBytes>256*1024*1024L)throw new IOException("decoded_texture_budget");
                String name="texture-"+textures.size()+".png";
                try(NativeImage image=new NativeImage(w,h,false)) { image.downloadTexture(0,false);image.writeToFile(out.resolve(name)); }
                return name;
            } finally { com.mojang.blaze3d.platform.GlStateManager._bindTexture(previous); }
        }
        void finish() throws Exception {
            for(Sink sink:sinks.values()) sink.finish();
            long total=0;try(Stream<Path> files=Files.list(out)) { for(Path p:files.toList()) total+=Files.size(p); }
            if(total>256*1024*1024L) throw new IOException("export_byte_limit");
        }
    }

    static final class Sink implements VertexConsumer {
        final Collector collector; final int material,object; final VertexFormat.Mode mode;
        final JsonArray positions=new JsonArray(),uv=new JsonArray(),colors=new JsonArray(),normals=new JsonArray(),indices=new JsonArray();
        float[] vertex;
        int count;
        Sink(Collector collector,int material,int object,VertexFormat.Mode mode) {this.collector=collector;this.material=material;this.object=object;this.mode=mode;}
        void commitVertex() {
            if(vertex==null)return;
            for(float f:vertex)if(!Float.isFinite(f))throw new IllegalStateException("nonfinite_vertex");
            for(int i=0;i<3;i++)positions.add(vertex[i]); for(int i=3;i<5;i++)uv.add(vertex[i]);
            for(int i=5;i<9;i++)colors.add(i<8 ? vertex[i]*vertex[i+7] : vertex[i]);for(int i=9;i<12;i++)normals.add(vertex[i]);
            count++; vertex=null;
        }
        public VertexConsumer addVertex(float x,float y,float z) {
            commitVertex(); if(++collector.vertices>200000)throw new IllegalStateException("vertex_limit");
            vertex=new float[]{x,y,z,0,0,1,1,1,1,0,1,0,1,1,1};return this;
        }
        public VertexConsumer setColor(int r,int g,int b,int a) {vertex[5]=r/255f;vertex[6]=g/255f;vertex[7]=b/255f;vertex[8]=a/255f;return this;}
        public VertexConsumer setUv(float u,float v) {vertex[3]=u;vertex[4]=v;return this;}
        public VertexConsumer setUv1(int u,int v) {if(u!=0||v!=10)throw new IllegalStateException("unsupported_overlay");return this;}
        public VertexConsumer setUv2(int u,int v) {
            NativeImage light=Minecraft.getInstance().gameRenderer.lightTexture().lightPixels;
            int color=light.getPixelRGBA(Math.clamp(u/16,0,15),Math.clamp(v/16,0,15));
            vertex[12]=(color&255)/255f;vertex[13]=((color>>8)&255)/255f;vertex[14]=((color>>16)&255)/255f;return this;
        }
        public VertexConsumer setNormal(float x,float y,float z) {vertex[9]=x;vertex[10]=y;vertex[11]=z;return this;}
        void finish() throws Exception {
            commitVertex();int stride=mode==VertexFormat.Mode.QUADS?4:3;
            if(count%stride!=0)throw new IOException("incomplete_primitive");
            if(count==0)return;
            for(int b=0;b<count;b+=stride) {indices.add(b);indices.add(b+1);indices.add(b+2);if(stride==4){indices.add(b);indices.add(b+2);indices.add(b+3);}}
            JsonObject mesh=new JsonObject();mesh.add("positions",positions);mesh.add("uv",uv);mesh.add("colors",colors);mesh.add("normals",normals);mesh.add("indices",indices);
            String name="mesh-"+collector.meshes.size()+".json";Files.writeString(collector.out.resolve(name),JSON.toJson(mesh));
            JsonObject item=new JsonObject();item.addProperty("file",name);item.addProperty("material",material);item.addProperty("object",object);item.addProperty("vertices",count);item.addProperty("triangles",indices.size()/3);collector.meshes.add(item);
        }
    }
}
