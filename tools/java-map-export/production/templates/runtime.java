    static final Gson JSON = Protocol.JSON;
    static Protocol protocol;
    static volatile String expectedContext;
    static volatile long expectedTick,prepareEpoch;
    static final java.util.concurrent.ScheduledExecutorService LIGHT_WORK=java.util.concurrent.Executors.newSingleThreadScheduledExecutor(task->{Thread thread=new Thread(task,"msc-capture-light-sync");thread.setDaemon(true);return thread;});
    static volatile boolean requested;
    static BlockPos low, high, contextLow, contextHigh;
    static final Map<String,CompoundTag> saved = new HashMap<>();

    static void tell(String message) {
        Minecraft mc=Minecraft.getInstance();
        if(message.startsWith("Refused: ")&&protocol!=null)protocol.failure(message.substring(9));
        mc.execute(()->{if(mc.player!=null)mc.player.displayClientMessage(Component.literal("MSC capture: "+message),false);});
    }
    static MinecraftServer server() throws Exception {
        Minecraft mc=Minecraft.getInstance();
        protocol=new Protocol(mc.gameDirectory.toPath()); protocol.verify();
        if(!protocol.request.get("minecraftVersion").getAsString().equals("@GAME@")||!protocol.request.get("loader").getAsString().equals("@LOADER@")||!protocol.request.get("loaderVersion").getAsString().equals("@VERSION@"))throw new IOException("incompatible_capture_helper");
        if(!net.minecraft.SharedConstants.getCurrentVersion().getName().equals("@GAME@")||!@ACTUAL_LOADER_VERSION@.equals("@VERSION@"))throw new IOException("incompatible_running_client");
        @VERIFY_RENDERER_VERSION@
        MinecraftServer server=mc.getSingleplayerServer();if(server==null)throw new IOException("open_dedicated_saved_world");
        Path world=server.getWorldPath(LevelResource.ROOT).toRealPath();
        if(!world.equals(protocol.game.resolve("saves/msc-capture").toRealPath()))throw new IOException("dedicated_saved_world_required");
        JsonObject area=protocol.request.getAsJsonObject("area"),context=protocol.request.getAsJsonObject("contextArea");
        low=position(area.getAsJsonArray("min"));high=position(area.getAsJsonArray("max"));contextLow=position(context.getAsJsonArray("min"));contextHigh=position(context.getAsJsonArray("max"));
        return server;
    }
    static BlockPos position(JsonArray array) {return new BlockPos(array.get(0).getAsInt(),array.get(1).getAsInt(),array.get(2).getAsInt());}
    static ServerLevel level(MinecraftServer server) throws IOException {
        ServerLevel level=server.getLevel(net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.DIMENSION,@RESOURCE@(protocol.request.get("dimension").getAsString())));
        if(level==null)throw new IOException("saved_dimension_missing");return level;
    }
    static CompoundTag savedChunk(int x,int z) throws Exception {
        String key=x+","+z;CompoundTag tag=saved.get(key);if(tag==null) {
            tag=@READ_NBT@;
            if(tag.getInt("xPos")!=x||tag.getInt("zPos")!=z||!List.of("full","minecraft:full").contains(tag.getString("Status")))throw new IOException("saved_chunk_context_invalid");saved.put(key,tag);
        }return tag;
    }
    static BlockState savedState(BlockPos p) throws Exception {
        CompoundTag chunk=savedChunk(p.getX()>>4,p.getZ()>>4);
        for(Tag sectionTag:chunk.getList("sections",10)) {
            CompoundTag section=(CompoundTag)sectionTag;if(section.getByte("Y")!=Math.floorDiv(p.getY(),16))continue;
            CompoundTag states=section.getCompound("block_states");ListTag palette=states.getList("palette",10);
            if(palette.isEmpty())throw new IOException("saved_palette_missing");int index=0;
            if(palette.size()>1){int bits=Math.max(4,32-Integer.numberOfLeadingZeros(palette.size()-1)),per=64/bits;
                long[] data=states.getLongArray("data");int flat=Math.floorMod(p.getY(),16)*256+Math.floorMod(p.getZ(),16)*16+Math.floorMod(p.getX(),16);
                if(data.length!=(4096+per-1)/per)throw new IOException("saved_palette_data_invalid");index=(int)((data[flat/per]>>>(flat%per*bits))&((1L<<bits)-1));
            }
            if(index>=palette.size())throw new IOException("saved_palette_index_invalid");
            CompoundTag definition=palette.getCompound(index);ResourceLocation id=@RESOURCE@(definition.getString("Name"));
            if(!BuiltInRegistries.BLOCK.containsKey(id))throw new IOException("saved_block_registry_missing");
            BlockState state=NbtUtils.readBlockState(BuiltInRegistries.BLOCK.asLookup(),definition);
            CompoundTag declared=definition.getCompound("Properties");JsonObject actual=properties(state);
            for(String key:declared.getAllKeys())if(!actual.has(key)||!actual.get(key).getAsString().equals(declared.getString(key)))throw new IOException("saved_block_property_missing");
            return state;
        }return net.minecraft.world.level.block.Blocks.AIR.defaultBlockState();
    }
    static boolean loaded(Level level,int x,int z) {return level.getChunkSource().getChunk(x,z,ChunkStatus.FULL,false)!=null;}
    static String canonical(Tag tag) {
        if(tag instanceof CompoundTag c){StringBuilder b=new StringBuilder("{");for(String key:new TreeSet<>(c.getAllKeys()))b.append(JSON.toJson(key)).append(':').append(canonical(c.get(key))).append(';');return b.append('}').toString();}
        if(tag instanceof ListTag l){StringBuilder b=new StringBuilder("[");for(Tag child:l)b.append(canonical(child)).append(';');return b.append(']').toString();}return tag.toString();
    }
    @SuppressWarnings({"unchecked","rawtypes"}) static String value(net.minecraft.world.level.block.state.properties.Property p,Comparable v){return p.getName(v);}
    static JsonObject properties(BlockState state){JsonObject fields=new JsonObject();state.getValues().entrySet().stream().sorted(Comparator.comparing(e->e.getKey().getName())).forEach(e->fields.addProperty(e.getKey().getName(),value(e.getKey(),e.getValue())));return fields;}
    static String context(Level level) throws Exception {
        StringBuilder data=new StringBuilder(level.dimension().location().toString());
        for(BlockPos p:BlockPos.betweenClosed(contextLow,contextHigh)){
            if(!loaded(level,p.getX()>>4,p.getZ()>>4))throw new IOException("unloaded_context_chunk");BlockState state=level.getBlockState(p);
            if(!state.equals(savedState(p)))throw new IOException("saved_state_changed");
            data.append(p.toShortString()).append(BuiltInRegistries.BLOCK.getKey(state.getBlock())).append(JSON.toJson(properties(state)));
            BlockEntity entity=level.getBlockEntity(p);if(entity!=null)data.append(canonical(@UPDATE_TAG@));
        }return Protocol.sha(data.toString().getBytes(StandardCharsets.UTF_8));
    }
    static void verifyResources() throws Exception {
        JsonObject winners=Protocol.json(protocol.root.resolve("winners.json"));
        Minecraft mc=Minecraft.getInstance();
        JsonArray selected=Protocol.json(protocol.root.resolve("prepared.json")).getAsJsonArray("selectedPacks");
        if(!JSON.toJsonTree(mc.options.resourcePacks).equals(selected))throw new IOException("selected_pack_order_changed");
        if(winners.size()>100000)throw new IOException("bound_resource_count_limit");
        long total=0;
        for(var entry:winners.entrySet()){
            String name=entry.getKey().substring("assets/".length());int split=name.indexOf('/');
            if(split<1)throw new IOException("invalid_bound_resource");
            ResourceLocation location=@RESOURCE@(name.substring(0,split)+":"+name.substring(split+1));
            MessageDigest digest=MessageDigest.getInstance("SHA-256");long resourceBytes=0;
            try(var stream=mc.getResourceManager().getResourceOrThrow(location).open()){
                byte[] buffer=new byte[65536];int count;
                while((count=stream.read(buffer))!=-1){resourceBytes+=count;total+=count;
                    if(resourceBytes>64*1024*1024L||total>512*1024*1024L)throw new IOException("bound_resource_byte_limit");digest.update(buffer,0,count);}
            }
            if(!HexFormat.of().formatHex(digest.digest()).equals(entry.getValue().getAsString()))throw new IOException("selected_resource_order_mismatch: "+location);
        }
    }
    static void synchronize(MinecraftServer server,ServerLevel level,long epoch) throws Exception {
        var engine=level.getChunkSource().getLightEngine();
        List<net.minecraft.world.level.chunk.LevelChunk> chunks=new ArrayList<>();
        List<java.util.concurrent.CompletableFuture<?>> work=new ArrayList<>();
        for(int x=contextLow.getX()>>4;x<=contextHigh.getX()>>4;x++)for(int z=contextLow.getZ()>>4;z<=contextHigh.getZ()>>4;z++){
            if(chunks.size()>=16)throw new IOException("saved_context_chunk_limit");
            var chunk=level.getChunkSource().getChunk(x,z,ChunkStatus.FULL,false);
            if(!(chunk instanceof net.minecraft.world.level.chunk.LevelChunk full))throw new IOException("unloaded_saved_context");
            chunks.add(full);work.add(engine.lightChunk(full,false));
        }
        var complete=java.util.concurrent.CompletableFuture.allOf(work.toArray(java.util.concurrent.CompletableFuture[]::new));
        Runnable poll=new Runnable(){int polls;
            public void run(){
                if(epoch!=prepareEpoch)return;
                try {
                    engine.tryScheduleUpdate();
                    if(!complete.isDone()){
                        if(++polls>3000)throw new IOException("saved_lighting_timeout");
                        LIGHT_WORK.schedule(()->server.tell(new net.minecraft.server.TickTask(server.getTickCount(),this)),10,java.util.concurrent.TimeUnit.MILLISECONDS);return;
                    }
                    complete.join();protocol.verify();
                    if(server.getPlayerList().getPlayers().size()!=1)throw new IOException("dedicated_single_player_required");
                    // Simulation is frozen, but packet queues still run. Whole
                    // saved chunk packets deliver states, light and update tags
                    // without depending on the suppressed chunk broadcast tick.
                    for(var chunk:chunks){
                        server.getPlayerList().broadcastAll(new net.minecraft.network.protocol.game.ClientboundLevelChunkWithLightPacket(chunk,engine,null,null),level.dimension());
                        for(BlockEntity entity:chunk.getBlockEntities().values()){
                            var update=entity.getUpdatePacket();if(update!=null)server.getPlayerList().broadcastAll(update,level.dimension());
                        }
                    }
                    server.getPlayerList().broadcastAll(new net.minecraft.network.protocol.game.ClientboundSetTimePacket(expectedTick,level.getDayTime(),level.getGameRules().getBoolean(net.minecraft.world.level.GameRules.RULE_DAYLIGHT)),level.dimension());
                    expectedContext=context(level);
                    Minecraft.getInstance().execute(()->{Minecraft.getInstance().pauseGame(false);tell("Saved context restored and synchronized. Once client updates arrive, close the pause screen and run /mscmapcapture export.");});
                }catch(Exception e){tell("Refused: "+e.getMessage());}
            }
        };
        poll.run();
    }
    int prepare() {
        try {
            MinecraftServer server=server();verifyResources();saved.clear();requested=false;expectedContext=null;FreezeGate.clear();long epoch=++prepareEpoch;
            server.execute(()->{
                try {
                    protocol.verify();ServerLevel level=level(server);
                    // Restore only the explicit saved context into the private copy.
                    // Mods never run against the ordinary client or managed world.
                    for(BlockPos p:BlockPos.betweenClosed(contextLow,contextHigh)){
                        if(!loaded(level,p.getX()>>4,p.getZ()>>4))throw new IOException("unloaded_saved_context: navigate the dedicated client to the requested saved area first");
                        level.setBlock(p,savedState(p),18);
                    }
                    for(int x=contextLow.getX()>>4;x<=contextHigh.getX()>>4;x++)for(int z=contextLow.getZ()>>4;z<=contextHigh.getZ()>>4;z++){
                        for(Tag entry:savedChunk(x,z).getList("block_entities",10)){
                            CompoundTag tag=(CompoundTag)entry;BlockPos p=new BlockPos(tag.getInt("x"),tag.getInt("y"),tag.getInt("z"));
                            if(p.getX()<contextLow.getX()||p.getX()>contextHigh.getX()||p.getY()<contextLow.getY()||p.getY()>contextHigh.getY()||p.getZ()<contextLow.getZ()||p.getZ()>contextHigh.getZ())continue;
                            BlockEntity entity=level.getBlockEntity(p);if(entity==null)throw new IOException("saved_block_entity_missing");
                            @LOAD_ENTITY@;entity.setChanged();level.sendBlockUpdated(p,level.getBlockState(p),level.getBlockState(p),3);
                        }
                    }
                    CompoundTag world=@READ_LEVEL_NBT@;
                    CompoundTag data=world.getCompound("Data");
                    server.getWorldData().overworldData().setGameTime(data.getLong("Time"));
                    server.getWorldData().overworldData().setDayTime(data.getLong("DayTime"));
                    var weather=server.getWorldData().overworldData();
                    weather.setRaining(data.getBoolean("raining"));weather.setThundering(data.getBoolean("thundering"));
                    weather.setRainTime(data.getInt("rainTime"));weather.setThunderTime(data.getInt("thunderTime"));weather.setClearWeatherTime(data.getInt("clearWeatherTime"));
                    level.setRainLevel(data.getBoolean("raining")?1:0);level.setThunderLevel(data.getBoolean("thundering")?1:0);
                    server.getPlayerList().broadcastAll(new net.minecraft.network.protocol.game.ClientboundGameEventPacket(data.getBoolean("raining")?net.minecraft.network.protocol.game.ClientboundGameEventPacket.START_RAINING:net.minecraft.network.protocol.game.ClientboundGameEventPacket.STOP_RAINING,0),level.dimension());
                    server.getPlayerList().broadcastAll(new net.minecraft.network.protocol.game.ClientboundGameEventPacket(net.minecraft.network.protocol.game.ClientboundGameEventPacket.RAIN_LEVEL_CHANGE,level.getRainLevel(0)),level.dimension());
                    server.getPlayerList().broadcastAll(new net.minecraft.network.protocol.game.ClientboundGameEventPacket(net.minecraft.network.protocol.game.ClientboundGameEventPacket.THUNDER_LEVEL_CHANGE,level.getThunderLevel(0)),level.dimension());
                    expectedTick=level.getGameTime();
                    @FREEZE@
                    synchronize(server,level,epoch);
                }catch(Exception e){tell("Refused: "+e.getMessage());}
            });
        }catch(Exception e){tell("Refused: "+e.getMessage());}return Command.SINGLE_SUCCESS;
    }
    int requestExport(){
        try{MinecraftServer server=server();@VERIFY_FREEZE@;if(expectedContext==null)throw new IOException("prepare_saved_context_first");if(Minecraft.getInstance().level.getGameTime()!=expectedTick||!context(Minecraft.getInstance().level).equals(expectedContext))throw new IOException("client_context_not_ready");
            if(!Minecraft.getInstance().level.isLightUpdateQueueEmpty()||Minecraft.getInstance().level.getChunkSource().getLightEngine().hasLightWork())throw new IOException("client_lighting_not_ready");
            Minecraft.getInstance().pauseGame(false);requested=true;
        }catch(Exception e){tell("Refused: "+e.getMessage());}return Command.SINGLE_SUCCESS;
    }
    void frame(org.joml.Matrix3f normal){
        if(!requested||!Minecraft.getInstance().isPaused())return;requested=false;
        try{exportFrame(normal);tell("Saved-frame bundle exported; import it into MSC and compare the named blocks.");}catch(Exception e){tell("Refused: "+e.getMessage());}
    }
    void exportFrame(org.joml.Matrix3f normal) throws Exception {
        Minecraft mc=Minecraft.getInstance();protocol.verify();verifyResources();
        if(mc.level==null||mc.level.getGameTime()!=expectedTick||!mc.level.dimension().location().toString().equals(protocol.request.get("dimension").getAsString())||!context(mc.level).equals(expectedContext))throw new IOException("client_context_changed");
        Path candidate=Files.createTempDirectory(protocol.root,"candidate-");Collector collector=null;
        try {
            Files.createDirectories(candidate.resolve("meshes"));Files.createDirectories(candidate.resolve("textures"));collector=new Collector(candidate,normal);
            for(BlockPos p:BlockPos.betweenClosed(low,high)){
                BlockState state=mc.level.getBlockState(p);String id=BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString();
                if(state.isAir()||id.equals("minecraft:water")||id.equals("minecraft:lava"))continue;
                if(collector.blocks.size()>=4096)throw new IOException("capture_position_limit");
                JsonObject block=new JsonObject();block.add("position",JSON.toJsonTree(new int[]{p.getX(),p.getY(),p.getZ()}));block.addProperty("id",id);block.add("state",properties(state));block.add("meshes",new JsonArray());
                collector.object=collector.blocks.size();collector.blocks.add(block);
                PoseStack pose=new PoseStack();pose.translate(p.getX(),p.getY(),p.getZ());
                var dispatcher=mc.getBlockRenderer();var model=dispatcher.getBlockModel(state);BlockEntity entity=mc.level.getBlockEntity(p);
                @RENDER_BLOCK@
                if(entity!=null)renderEntity(entity,pose,collector);
            }
            if(collector.blocks.isEmpty())throw new IOException("capture_positions_empty");
            collector.finish();protocol.verify();if(!context(mc.level).equals(expectedContext))throw new IOException("client_context_changed");
            JsonObject result=new JsonObject();result.add("request",protocol.request);result.addProperty("observedSnapshotId",protocol.snapshot());result.addProperty("observedInputFingerprint",protocol.inputs());
            result.addProperty("capturedAtUnix",java.time.Instant.now().getEpochSecond());result.addProperty("gameTick",mc.level.getGameTime());result.addProperty("savedFrame",true);
            result.add("directionalLights",collector.lights);result.add("materials",collector.materials);result.add("blocks",collector.blocks);protocol.publish(candidate,result);
        }finally{if(collector!=null)collector.close();try(var paths=Files.walk(candidate)){for(Path p:paths.sorted(Comparator.reverseOrder()).toList())Files.deleteIfExists(p);}}
    }
