    @SuppressWarnings({"unchecked","rawtypes"})
    static void renderEntity(BlockEntity entity,PoseStack pose,Collector collector) throws IOException {
        var dispatcher=Minecraft.getInstance().getBlockEntityRenderDispatcher();
        BlockEntityRenderer renderer=dispatcher.getRenderer(entity);
        if(renderer==null) {
            if(entity.getBlockState().getRenderShape()==RenderShape.MODEL)return;
            throw new IOException("block_entity_renderer_missing");
        }
        // Invoke the actual registered renderer with this position's client-visible contents.
        // Direct invocation avoids silently skipping an object due to the current camera's range.
        renderer.render(entity,0,pose,collector,LevelRenderer.getLightColor(entity.getLevel(),entity.getBlockPos()),OverlayTexture.NO_OVERLAY);
    }

    static final class Collector implements MultiBufferSource {
        final Path out;
        final Map<String,Sink> sinks=new LinkedHashMap<>();
        final Map<ResourceLocation,NativeImage> textures=new LinkedHashMap<>();
        final Map<String,Crop> crops=new LinkedHashMap<>();
        final Map<String,Integer> materialIds=new LinkedHashMap<>();
        long sourceImageBytes;
        final JsonArray materials=new JsonArray(),meshes=new JsonArray(),lights=new JsonArray();
        int object,vertices;long decodedImageBytes; final JsonArray blocks=new JsonArray();
        Collector(Path out,org.joml.Matrix3f normal) throws IOException {
            this.out=out;
            org.joml.Matrix3f inverse=new org.joml.Matrix3f(normal).invert();
            for(org.joml.Vector3f direction:com.mojang.blaze3d.systems.RenderSystem.shaderLightDirections) {
                if(direction==null)throw new IOException("directional_light_context_missing");
                org.joml.Vector3f world=inverse.transform(new org.joml.Vector3f(direction));
                lights.add(JSON.toJsonTree(List.of(world.x,world.y,world.z)));
            }
        }
        public VertexConsumer getBuffer(RenderType type) {
            try {
                if(type.mode()!=VertexFormat.Mode.QUADS&&type.mode()!=VertexFormat.Mode.TRIANGLES) throw new IOException("unsupported_topology: "+type.name);
                // Capture only material states represented by the portable format.
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
                String key=object+":"+type.toString();
                Sink sink=sinks.get(key);
                if(sink==null) {
                    JsonObject material=new JsonObject(); material.addProperty("mode",mode);
                    material.addProperty("alphaThreshold",mode.equals("cutout")?0.1:0);
                    material.addProperty("cull",!type.name.contains("no_cull")&&!type.name.equals("entity_translucent"));
                    material.addProperty("depthWrite",mode.equals("opaque")||mode.equals("cutout"));
                    material.addProperty("directionalLighting",type.name.startsWith("entity_")||type.name.equals("item_entity_translucent_cull"));

                    sink=new Sink(this,material,location,object,type.mode());sinks.put(key,sink);
                }
                return sink;
            } catch(Exception e) { throw new IllegalStateException(e.getMessage(),e); }
        }
        record Crop(String file,int x,int y,int width,int height,int sourceWidth,int sourceHeight) {}
        NativeImage sourceTexture(ResourceLocation location) throws Exception {
            NativeImage existing=textures.get(location);if(existing!=null)return existing;
            if(textures.size()>=128)throw new IOException("texture_count_limit");
            int previous=GL11.glGetInteger(GL11.GL_TEXTURE_BINDING_2D);
            try {
                Minecraft.getInstance().getTextureManager().getTexture(location).bind();
                int width=GL11.glGetTexLevelParameteri(GL11.GL_TEXTURE_2D,0,GL11.GL_TEXTURE_WIDTH);
                int height=GL11.glGetTexLevelParameteri(GL11.GL_TEXTURE_2D,0,GL11.GL_TEXTURE_HEIGHT);
                if(width<1||height<1||width>8192||height>8192)throw new IOException("source_texture_size_limit");
                long bytes=(long)width*height*4;
                if(sourceImageBytes+bytes>256*1024*1024L)throw new IOException("source_texture_budget");
                NativeImage image=new NativeImage(width,height,false);
                try{image.downloadTexture(0,false);}catch(Throwable e){image.close();throw e;}
                textures.put(location,image);sourceImageBytes+=bytes;return image;
            } finally { com.mojang.blaze3d.platform.GlStateManager._bindTexture(previous); }
        }
        Crop crop(ResourceLocation location,JsonArray uv,int base,int count) throws Exception {
            NativeImage source=sourceTexture(location);int sw=source.getWidth(),sh=source.getHeight();
            float minU=1,minV=1,maxU=0,maxV=0;
            for(int i=base;i<base+count;i++) {
                float u=uv.get(i*2).getAsFloat(),v=uv.get(i*2+1).getAsFloat();
                if(u<0||u>1||v<0||v>1)throw new IOException("unsupported_repeating_texture_uv");
                minU=Math.min(minU,u);minV=Math.min(minV,v);maxU=Math.max(maxU,u);maxV=Math.max(maxV,v);
            }
            // Keep a one-pixel apron so linear filtering retains the source
            // samples, without shrinking or resampling a large texture atlas.
            int x=Math.max(0,(int)Math.floor(minU*sw)-1),y=Math.max(0,(int)Math.floor(minV*sh)-1);
            int w=Math.min(sw,(int)Math.ceil(maxU*sw)+1)-x,h=Math.min(sh,(int)Math.ceil(maxV*sh)+1)-y;
            if(w<1||h<1||w>4096||h>4096)throw new IOException("primitive_texture_crop_limit");
            String key=location+":"+x+","+y+","+w+","+h;Crop existing=crops.get(key);if(existing!=null)return existing;
            long bytes=(long)w*h*4;if(decodedImageBytes+bytes>128*1024*1024L)throw new IOException("decoded_texture_budget");
            if(crops.size()>=1024)throw new IOException("texture_crop_count_limit");
            String file="textures/texture-"+crops.size()+".png";
            try(NativeImage image=new NativeImage(w,h,false)) {
                for(int py=0;py<h;py++)for(int px=0;px<w;px++)image.setPixelRGBA(px,py,source.getPixelRGBA(x+px,y+py));
                image.writeToFile(out.resolve(file));
            }
            Crop crop=new Crop(file,x,y,w,h,sw,sh);crops.put(key,crop);decodedImageBytes+=bytes;return crop;
        }
        int material(JsonObject prototype,Crop crop) throws IOException {
            JsonObject material=prototype.deepCopy();material.addProperty("texture",crop.file());String key=JSON.toJson(material);
            Integer existing=materialIds.get(key);if(existing!=null)return existing;
            if(materials.size()>=1024)throw new IOException("material_count_limit");
            int id=materials.size();materials.add(material);materialIds.put(key,id);return id;
        }
        void close(){for(NativeImage image:textures.values())image.close();textures.clear();}
        void finish() throws Exception {
            try {for(Sink sink:sinks.values()) sink.finish();} finally {close();}
            long total=0;try(Stream<Path> files=Files.walk(out)) { for(Path p:files.filter(Files::isRegularFile).toList()) total+=Files.size(p); }
            if(total>64*1024*1024L) throw new IOException("export_byte_limit");
        }
    }

    static final class Sink implements VertexConsumer {
        final Collector collector; final JsonObject prototype; final ResourceLocation location; final int object; final VertexFormat.Mode mode;
        final JsonArray positions=new JsonArray(),uv=new JsonArray(),colors=new JsonArray(),normals=new JsonArray(),indices=new JsonArray();
        float[] vertex;
        int count;
        Sink(Collector collector,JsonObject prototype,ResourceLocation location,int object,VertexFormat.Mode mode) {this.collector=collector;this.prototype=prototype;this.location=location;this.object=object;this.mode=mode;}
        void commitVertex() {
            if(vertex==null)return;
            for(float f:vertex)if(!Float.isFinite(f))throw new IllegalStateException("nonfinite_vertex");
            for(int i=0;i<3;i++)positions.add(vertex[i]); for(int i=3;i<5;i++)uv.add(vertex[i]);
            for(int i=5;i<9;i++)colors.add(i<8 ? vertex[i]*vertex[i+7] : vertex[i]);for(int i=9;i<12;i++)normals.add(vertex[i]);
            count++; vertex=null;
        }
        public VertexConsumer vertex(double x,double y,double z) {
            commitVertex(); if(++collector.vertices>1000000)throw new IllegalStateException("vertex_limit");
            vertex=new float[]{(float)x,(float)y,(float)z,0,0,1,1,1,1,0,1,0,1,1,1};return this;
        }
        public VertexConsumer color(int r,int g,int b,int a) {vertex[5]=r/255f;vertex[6]=g/255f;vertex[7]=b/255f;vertex[8]=a/255f;return this;}
        public VertexConsumer uv(float u,float v) {vertex[3]=u;vertex[4]=v;return this;}
        public VertexConsumer overlayCoords(int u,int v) {if(u!=0||v!=10)throw new IllegalStateException("unsupported_overlay");return this;}
        public VertexConsumer uv2(int u,int v) {
            NativeImage light=Minecraft.getInstance().gameRenderer.lightTexture().lightPixels;
            int color=light.getPixelRGBA(Math.max(0,Math.min(15,u/16)),Math.max(0,Math.min(15,v/16)));
            vertex[12]=(color&255)/255f;vertex[13]=((color>>8)&255)/255f;vertex[14]=((color>>16)&255)/255f;return this;
        }
        public VertexConsumer normal(float x,float y,float z) {vertex[9]=x;vertex[10]=y;vertex[11]=z;return this;}
        public void endVertex() { commitVertex(); }
        public void defaultColor(int r,int g,int b,int a) { throw new IllegalStateException("unsupported_default_color"); }
        public void unsetDefaultColor() {}
        void finish() throws Exception {
            commitVertex();int stride=mode==VertexFormat.Mode.QUADS?4:3;
            if(count%stride!=0)throw new IOException("incomplete_primitive");
            if(count==0)return;
            final class Mesh {
                final JsonArray p=new JsonArray(),u=new JsonArray(),c=new JsonArray(),n=new JsonArray(),indices=new JsonArray();int vertices;
            }
            Map<Integer,Mesh> meshes=new LinkedHashMap<>();
            for(int b=0;b<count;b+=stride) {
                Collector.Crop crop=collector.crop(location,uv,b,stride);int material=collector.material(prototype,crop);
                Mesh mesh=meshes.computeIfAbsent(material,key->new Mesh());int start=mesh.vertices;
                for(int i=b;i<b+stride;i++) {
                    for(int j=0;j<3;j++){mesh.p.add(positions.get(i*3+j));mesh.n.add(normals.get(i*3+j));}
                    for(int j=0;j<4;j++)mesh.c.add(colors.get(i*4+j));
                    mesh.u.add((uv.get(i*2).getAsFloat()*crop.sourceWidth()-crop.x())/crop.width());
                    mesh.u.add((uv.get(i*2+1).getAsFloat()*crop.sourceHeight()-crop.y())/crop.height());mesh.vertices++;
                }
                mesh.indices.add(start);mesh.indices.add(start+1);mesh.indices.add(start+2);
                if(stride==4){mesh.indices.add(start);mesh.indices.add(start+2);mesh.indices.add(start+3);}
            }
            for(var entry:meshes.entrySet()) {
                Mesh arrays=entry.getValue();JsonObject mesh=new JsonObject();mesh.add("positions",arrays.p);mesh.add("uv",arrays.u);mesh.add("colors",arrays.c);mesh.add("normals",arrays.n);mesh.add("indices",arrays.indices);
                if(collector.meshes.size()>=4095)throw new IOException("mesh_file_count_limit");
                String name="meshes/mesh-"+collector.meshes.size()+".json";Files.writeString(collector.out.resolve(name),JSON.toJson(mesh));
                if(collector.blocks.get(object).getAsJsonObject().getAsJsonArray("meshes").size()>=128)throw new IOException("block_mesh_count_limit");
                JsonObject item=new JsonObject();item.addProperty("file",name);item.addProperty("material",entry.getKey());collector.blocks.get(object).getAsJsonObject().getAsJsonArray("meshes").add(item);collector.meshes.add(item);
            }
        }
    }
}
