package dev.msc.mapexport;

import com.google.gson.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.zip.*;

/** Data-only correspondence checks shared by all loader adapters. */
public final class Protocol {
    public static final Gson JSON = new GsonBuilder().disableHtmlEscaping().create();
    public final Path root, game;
    public final JsonObject request;
    private final JsonObject receipts;
    public Protocol(Path game) throws Exception {
        this.game = game.toRealPath(); root = this.game.resolve(".msc-map-capture");
        if (Files.isSymbolicLink(root) || !Files.isRegularFile(root.resolve("prepared.json")))
            throw new IOException("dedicated_capture_instance_required");
        request = json(root.resolve("request.json"));
        receipts = json(root.resolve("prepared.json")).getAsJsonObject("files");
        if (!request.get("format").getAsString().equals("msc-contextual-mesh-1")) throw new IOException("unsupported_capture_format");
        verifyFiles();
    }
    public static JsonObject json(Path file) throws Exception {
        byte[] raw = read(file,8*1024*1024); return JsonParser.parseString(new String(raw,StandardCharsets.UTF_8)).getAsJsonObject();
    }
    public static byte[] read(Path file,int limit) throws Exception {
        Path current=file.toAbsolutePath();
        while(current!=null) { if(Files.isSymbolicLink(current)) throw new IOException("linked_capture_input"); current=current.getParent(); }
        if(!Files.isRegularFile(file)||Files.size(file)>limit) throw new IOException("capture_input_byte_limit");
        try(InputStream in=Files.newInputStream(file)) { byte[] bytes=in.readNBytes(limit+1); if(bytes.length>limit)throw new IOException("capture_input_byte_limit");return bytes; }
    }
    public static String sha(byte[] raw) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(raw)); }
    private static String fileSha(Path file,long limit) throws Exception {
        Path current=file.toAbsolutePath();
        while(current!=null){if(Files.isSymbolicLink(current))throw new IOException("linked_capture_input");current=current.getParent();}
        if(!Files.isRegularFile(file)||Files.size(file)>limit)throw new IOException("capture_input_byte_limit");
        MessageDigest digest=MessageDigest.getInstance("SHA-256");long bytes=0;
        try(var in=Files.newInputStream(file)){byte[] buffer=new byte[65536];int count;
            while((count=in.read(buffer))!=-1){bytes+=count;if(bytes>limit)throw new IOException("capture_input_byte_limit");digest.update(buffer,0,count);}}
        return HexFormat.of().formatHex(digest.digest());
    }
    private void verifyFiles() throws Exception {
        if(receipts.size()>10000)throw new IOException("capture_input_count_limit");
        for(var entry:receipts.entrySet()) {
            String name=entry.getKey();
            if(name.startsWith("/")||name.contains("\\")||Arrays.asList(name.split("/")).contains(".."))throw new IOException("unsafe_capture_input");
            Path file=game.resolve(name).normalize(); if(!file.startsWith(game))throw new IOException("unsafe_capture_input");
            if(!fileSha(file,256*1024*1024).equals(entry.getValue().getAsString()))throw new IOException("capture_input_changed: "+name);
        }
        // No extra active mod or pack may silently alter the bound renderer.
        for(String folder:List.of("mods","resourcepacks","config")) {
            Path directory=game.resolve(folder); if(!Files.exists(directory))continue;
            try(var paths=Files.walk(directory)) { for(Path p:paths.filter(Files::isRegularFile).toList()) {
                String name=game.relativize(p).toString().replace('\\','/');
                if(!receipts.has(name))throw new IOException("unbound_client_input: "+name);
            } }
        }
    }
    public byte[] chunk(int x,int z) throws Exception {
        String dim=request.get("dimension").getAsString(); String folder=switch(dim) {
            case "minecraft:overworld" -> "region"; case "minecraft:the_nether" -> "DIM-1/region"; case "minecraft:the_end" -> "DIM1/region";
            default -> "dimensions/"+dim.replace(':','/')+"/region";
        };
        Path file=root.resolve("snapshot/"+folder+"/r."+Math.floorDiv(x,32)+"."+Math.floorDiv(z,32)+".mca");
        byte[] region=read(file,256*1024*1024); int index=(Math.floorMod(x,32)+Math.floorMod(z,32)*32)*4;
        if(region.length<8192)throw new IOException("invalid_region_header");
        int offset=((region[index]&255)<<16)|((region[index+1]&255)<<8)|(region[index+2]&255), sectors=region[index+3]&255;
        if(offset<2||sectors==0||(long)(offset+sectors)*4096>region.length)throw new IOException("missing_saved_chunk");
        int start=offset*4096, size=((region[start]&255)<<24)|((region[start+1]&255)<<16)|((region[start+2]&255)<<8)|(region[start+3]&255);
        if(size<1||size+4>sectors*4096)throw new IOException("invalid_region_record");
        InputStream raw=new ByteArrayInputStream(region,start+5,size-1);
        InputStream decoder=switch(region[start+4]) {case 1->new GZIPInputStream(raw);case 2->new InflaterInputStream(raw);case 3->raw;default->throw new IOException("unsupported_chunk_compression");};
        try(decoder) {byte[] bytes=decoder.readNBytes(8*1024*1024+1);if(bytes.length>8*1024*1024)throw new IOException("chunk_byte_limit");return bytes;}
    }
    public String snapshot() throws Exception {
        TreeMap<String,String> hashes=new TreeMap<>(); hashes.put("level.dat",sha(read(root.resolve("snapshot/level.dat"),8*1024*1024)));
        JsonObject area=request.getAsJsonObject("contextArea"); int[] min=JSON.fromJson(area.get("min"),int[].class),max=JSON.fromJson(area.get("max"),int[].class);
        for(int x=Math.floorDiv(min[0],16);x<=Math.floorDiv(max[0],16);x++)for(int z=Math.floorDiv(min[2],16);z<=Math.floorDiv(max[2],16);z++)hashes.put(x+","+z,sha(chunk(x,z)));
        JsonArray identity=new JsonArray();identity.add(request.get("dimension"));
        // Rust's Area serializer has a fixed min/max field order.
        JsonObject canonicalArea=new JsonObject();canonicalArea.add("min",area.get("min"));canonicalArea.add("max",area.get("max"));identity.add(canonicalArea);identity.add(JSON.toJsonTree(hashes));
        return sha(JSON.toJson(identity).getBytes(StandardCharsets.UTF_8));
    }
    public String inputs() throws Exception {
        verifyFiles();
        // resources.json is the typed host manifest. Preparation verified every
        // active source against actual selected client bytes before binding it.
        return sha(read(root.resolve("resources.json"),8*1024*1024));
    }
    public void verify() throws Exception {
        if(!snapshot().equals(request.get("snapshotId").getAsString()))throw new IOException("snapshot_mismatch");
        if(!inputs().equals(request.get("inputFingerprint").getAsString()))throw new IOException("client_inputs_mismatch");
    }
    public void failure(String message) {
        try {
            String prefix=message.split(":",2)[0];
            String code=prefix.matches("[a-z_]{1,96}")?prefix:"capture_failed";
            JsonObject result=new JsonObject();result.addProperty("code",code);
            // Persist controlled explanations only: arbitrary mod exceptions can
            // contain private configuration values or credentials.
            String detail=switch(code){
                case "unloaded_saved_context","unloaded_context_chunk" -> "Navigate the dedicated client to the requested saved area before preparing.";
                case "saved_freeze_not_observed" -> "Allow the dedicated client one frame before exporting.";
                case "client_context_not_ready","client_context_changed","saved_state_changed" -> "The client has not received the restored saved context. Prepare again and wait for updates.";
                case "selected_resource_order_mismatch","selected_pack_order_changed" -> "The active Minecraft resources differ from the imported client selection. Reimport matching resources.";
                case "unsupported_active_fabric_renderer","unsupported_custom_material_state","unsupported_material","unsupported_topology" -> "This renderer or material cannot be represented by the current capture format.";
                case "incompatible_capture_helper" -> "Use the exact Minecraft and loader versions listed by the helper manifest.";
                case "open_dedicated_saved_world","dedicated_saved_world_required" -> "Open the private msc-capture saved world in the prepared instance.";
                default -> "Capture was refused. The previous complete output remains available; inspect the named refusal in the dedicated Minecraft client.";
            };
            result.addProperty("detail",detail);result.addProperty("failedAtUnix",java.time.Instant.now().getEpochSecond());
            result.add("request",request);Path candidate=root.resolve("failure-"+UUID.randomUUID()+".part");
            try{Files.writeString(candidate,JSON.toJson(result),StandardCharsets.UTF_8,StandardOpenOption.CREATE_NEW);
                Files.move(candidate,root.resolve("failure.json"),StandardCopyOption.ATOMIC_MOVE,StandardCopyOption.REPLACE_EXISTING);
            }finally{Files.deleteIfExists(candidate);}
        }catch(Exception ignored){/* The client chat still reports the refusal. */}
    }
    public void publish(Path candidate,JsonObject manifest) throws Exception {
        verify(); JsonObject files=new JsonObject();long total=0;
        try(var paths=Files.walk(candidate)) {for(Path p:paths.filter(Files::isRegularFile).sorted().toList()) {
            if(files.size()>=4096)throw new IOException("capture_file_count_limit");
            byte[] raw=read(p,16*1024*1024);total+=raw.length;if(total>64*1024*1024)throw new IOException("capture_byte_limit");
            JsonObject receipt=new JsonObject();receipt.addProperty("sha256",sha(raw));receipt.addProperty("bytes",raw.length);
            files.add(candidate.relativize(p).toString().replace('\\','/'),receipt);
        }}
        manifest.add("files",files);Files.writeString(candidate.resolve("capture.json"),JSON.toJson(manifest),StandardCharsets.UTF_8);
        Path exports=root.resolve("exports");Files.createDirectories(exports);Path part=exports.resolve(UUID.randomUUID()+".part"),output=exports.resolve("capture-"+UUID.randomUUID()+".zip");
        try {try(ZipOutputStream zip=new ZipOutputStream(Files.newOutputStream(part,StandardOpenOption.CREATE_NEW))) {
            try(var paths=Files.walk(candidate)){for(Path p:paths.filter(Files::isRegularFile).sorted().toList()) {
                String name=candidate.relativize(p).toString().replace('\\','/');zip.putNextEntry(new ZipEntry(name));zip.write(read(p,16*1024*1024));zip.closeEntry();
            }}
        }
        if(Files.size(part)>64*1024*1024)throw new IOException("capture_byte_limit");
        Files.move(part,output,StandardCopyOption.ATOMIC_MOVE);
        Path next=exports.resolve("current-"+UUID.randomUUID());Files.writeString(next,output.getFileName().toString());
        Files.move(next,exports.resolve("current"),StandardCopyOption.REPLACE_EXISTING,StandardCopyOption.ATOMIC_MOVE);
        Files.deleteIfExists(root.resolve("failure.json"));
        // Retain a bounded set of complete immutable outputs for offline retry.
        try(var paths=Files.list(exports)) {
            var saved=paths.filter(p->p.getFileName().toString().startsWith("capture-")&&p.getFileName().toString().endsWith(".zip"))
                .sorted(Comparator.comparingLong((Path p)->{try{return Files.getLastModifiedTime(p).toMillis();}catch(IOException e){return 0L;}}).reversed()).toList();
            int count=0;long bytes=0;for(Path p:saved){count++;bytes+=Files.size(p);if((count>8||bytes>512*1024*1024L)&&!p.equals(output))Files.deleteIfExists(p);}
        }
        } finally {Files.deleteIfExists(part);}
    }
}
