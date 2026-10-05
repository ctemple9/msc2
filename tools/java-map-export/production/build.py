#!/usr/bin/env python3
"""Build pinned data-export adapters. Never runs tests, clients or fixtures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent
GRADLE_HASHES = {'8.8': 'a4b4158601f8636cdeeab09bd76afb640030bb5b144aafe261a5e8af027dc612', '9.2.1': '72f44c9f8ebcb1af43838f45ee5c4aa9c5444898b3468ab3f4af7b6076c5bc3f'}
PINS = {
    'forge': {'minecraft': '1.20.1', 'loader': '47.4.10', 'java': 17, 'gradle': '8.8', 'plugin': '6.0.54'},
    'neoforge': {'minecraft': '1.21.1', 'loader': '21.1.251', 'java': 21, 'gradle': '9.2.1', 'plugin': '2.0.141'},
    'fabric': {'minecraft': '1.20.1', 'loader': '0.16.14', 'java': 17, 'gradle': '8.8', 'plugin': '1.6.12', 'api': '0.92.5+1.20.1'},
}


def java_home(major, explicit):
    candidates = [explicit, os.environ.get(f'JAVA{major}_HOME'), os.environ.get(f'JAVA_HOME_{major}_X64'), os.environ.get(f'JAVA_HOME_{major}_ARM64'), os.environ.get('JAVA_HOME')]
    if os.name != 'nt':
        candidates += [f'/usr/lib/jvm/temurin-{major}-jdk', f'/usr/lib/jvm/java-{major}-openjdk']
        candidates += [str(p.parent.parent) for p in Path('/Library/Java/JavaVirtualMachines').glob('*/Contents/Home/bin/java')]
    for home in candidates:
        if not home:
            continue
        binary = Path(home) / 'bin' / ('java.exe' if os.name == 'nt' else 'java')
        if not binary.is_file():
            continue
        result = subprocess.run([str(binary), '-version'], capture_output=True, text=True, check=False)
        if result.returncode == 0 and f'"{major}.' in result.stderr:
            return str(Path(home).resolve())
    raise ValueError(f'Java {major} JDK required; provide --java{major}-home. No game launch attempted.')


def gradle(version, explicit):
    candidates = [explicit]
    candidates += [str(p) for p in (Path.home()/'.gradle/wrapper/dists'/f'gradle-{version}-bin').glob(f'*/gradle-{version}/bin/'+('gradle.bat' if os.name == 'nt' else 'gradle'))]
    candidates += [str(Path.home()/f'.cache/msc-map-forge-capture-inputs/gradle-{version}/bin/gradle')]
    candidates += [shutil.which('gradle')]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():

            if f'gradle-{version}' in str(candidate) or explicit == candidate:
                return str(Path(candidate).resolve())
    cache=Path(os.environ.get('MSC2_BUILD_CACHE',str(Path.home()/'.cache/msc-map-capture-build')))
    executable=cache/f'gradle-{version}'/'bin'/('gradle.bat' if os.name == 'nt' else 'gradle')
    if executable.is_file():return str(executable)
    cache.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='gradle-',dir=cache) as temporary:
        archive=Path(temporary)/'gradle.zip';digest=hashlib.sha256();total=0
        with urllib.request.urlopen(f'https://downloads.gradle.org/distributions/gradle-{version}-bin.zip',timeout=60) as response,archive.open('wb') as output:
            while True:
                chunk=response.read(1024*1024)
                if not chunk:break
                total+=len(chunk)
                if total>256*1024*1024:raise ValueError('Gradle archive exceeds the build-tool budget')
                output.write(chunk);digest.update(chunk)
        if digest.hexdigest()!=GRADLE_HASHES[version]:raise ValueError('Official Gradle distribution checksum mismatch')
        with zipfile.ZipFile(archive) as zip:
            for member in zip.infolist():
                name=Path(member.filename)
                if name.is_absolute() or '..' in name.parts or not str(name).startswith(f'gradle-{version}/'):raise ValueError('Unsafe Gradle distribution member')
            zip.extractall(cache)
        if os.name!='nt':executable.chmod(0o755)
    return str(executable)


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding='utf-8')


def generate(loader, directory):
    pin = PINS[loader]
    source = ROOT.parents[1] / ('world-map-proof/client-capture/src/main/java/dev/msc/mapproof/capture/CaptureProof.java' if loader == 'neoforge' else 'java-map-export/forge-1.20.1/src/main/java/dev/msc/mapexport/capture/CaptureFixture.java')
    header = source.read_text().split('@Mod(')[0].replace('dev.msc.mapproof.capture', 'dev.msc.mapexport').replace('dev.msc.mapexport.capture', 'dev.msc.mapexport')
    if loader == 'fabric':
        header = '\n'.join(line for line in header.splitlines() if 'net.minecraftforge' not in line) + '\n'
        register = '''public final class CaptureClient implements net.fabricmc.api.ClientModInitializer {
    public void onInitializeClient() {
        net.fabricmc.fabric.api.client.command.v2.ClientCommandRegistrationCallback.EVENT.register((dispatcher,registry)->dispatcher.register(net.fabricmc.fabric.api.client.command.v2.ClientCommandManager.literal("mscmapcapture")
          .then(net.fabricmc.fabric.api.client.command.v2.ClientCommandManager.literal("prepare").executes(c->prepare()))
          .then(net.fabricmc.fabric.api.client.command.v2.ClientCommandManager.literal("export").executes(c->requestExport()))));
        net.fabricmc.fabric.api.client.rendering.v1.WorldRenderEvents.END.register(context->frame(new org.joml.Matrix3f(context.matrixStack().last().normal())));
    }
'''
    else:
        annotation = '@Mod(value="msc_map_capture",dist=Dist.CLIENT)' if loader == 'neoforge' else '@Mod("msc_map_capture")'
        bus = 'NeoForge' if loader == 'neoforge' else 'MinecraftForge'
        ctor = 'IEventBus bus' if loader == 'neoforge' else ''
        register = f'''{annotation}
public final class CaptureClient {{
    public CaptureClient({ctor}) {{
        {bus}.EVENT_BUS.addListener(this::commands);{bus}.EVENT_BUS.addListener(this::onFrame);
    }}
    void commands(RegisterClientCommandsEvent event) {{ event.getDispatcher().register(literal("mscmapcapture")
      .then(literal("prepare").executes(c->prepare())).then(literal("export").executes(c->requestExport()))); }}
    void onFrame(RenderLevelStageEvent event) {{ if(event.getStage()==RenderLevelStageEvent.Stage.AFTER_LEVEL)frame(new org.joml.Matrix3f(event.getPoseStack().last().normal())); }}
'''
    runtime = (ROOT/'templates/runtime.java').read_text()
    replacements = {
        '@GAME@': pin['minecraft'], '@LOADER@': loader, '@VERSION@': pin['loader'],
        '@ACTUAL_LOADER_VERSION@': 'net.fabricmc.loader.api.FabricLoader.getInstance().getModContainer("fabricloader").orElseThrow().getMetadata().getVersion().getFriendlyString()' if loader == 'fabric' else 'net.neoforged.neoforge.internal.versions.neoforge.NeoForgeVersion.getVersion()' if loader == 'neoforge' else 'net.minecraftforge.versions.forge.ForgeVersion.getVersion()',
        '@VERIFY_RENDERER_VERSION@': f'if(!net.fabricmc.loader.api.FabricLoader.getInstance().getModContainer("fabric-api").orElseThrow().getMetadata().getVersion().getFriendlyString().equals("{pin["api"]}"))throw new IOException("incompatible_running_fabric_api");' if loader == 'fabric' else '',
        '@RESOURCE@': 'ResourceLocation.parse' if loader == 'neoforge' else 'new ResourceLocation',
        '@READ_NBT@': 'NbtIo.read(new java.io.DataInputStream(new java.io.ByteArrayInputStream(protocol.chunk(x,z))), NbtAccounter.unlimitedHeap())' if loader == 'neoforge' else 'NbtIo.read(new java.io.DataInputStream(new java.io.ByteArrayInputStream(protocol.chunk(x,z))))',
        '@READ_LEVEL_NBT@': 'NbtIo.read(new java.io.DataInputStream(new java.util.zip.GZIPInputStream(new java.io.ByteArrayInputStream(Protocol.read(protocol.root.resolve("snapshot/level.dat"),8*1024*1024)))), NbtAccounter.unlimitedHeap())' if loader == 'neoforge' else 'NbtIo.read(new java.io.DataInputStream(new java.util.zip.GZIPInputStream(new java.io.ByteArrayInputStream(Protocol.read(protocol.root.resolve("snapshot/level.dat"),8*1024*1024)))))',
        '@UPDATE_TAG@': 'entity.getUpdateTag(level.registryAccess())' if loader == 'neoforge' else 'entity.getUpdateTag()',
        '@LOAD_ENTITY@': 'entity.loadWithComponents(tag,level.registryAccess())' if loader == 'neoforge' else 'entity.load(tag)',
        '@FREEZE@': 'server.tickRateManager().setFrozen(true);' if loader == 'neoforge' else 'FreezeGate.freeze(server);',
        '@VERIFY_FREEZE@': 'if(!server.tickRateManager().isFrozen())throw new IOException("saved_context_not_frozen");' if loader == 'neoforge' else 'if(!FreezeGate.observed(server))throw new IOException("saved_freeze_not_observed: allow the dedicated client one frame before export");',
    }
    if loader == 'fabric':
        render = '''if(state.getRenderShape()==RenderShape.MODEL) {
                    if(!((net.fabricmc.fabric.api.renderer.v1.model.FabricBakedModel)model).isVanillaAdapter()) {
                        var active=net.fabricmc.fabric.api.renderer.v1.RendererAccess.INSTANCE.getRenderer();
                        if(active==null||!active.getClass().getName().equals("net.fabricmc.fabric.impl.client.indigo.renderer.IndigoRenderer"))throw new IOException("unsupported_active_fabric_renderer");
                        final Collector buffers=collector;
                        var context=new net.fabricmc.fabric.impl.client.indigo.renderer.render.BlockRenderContext() {
                            protected VertexConsumer getVertexConsumer(RenderType layer){return buffers.getBuffer(layer);}
                        };
                        context.render(mc.level,model,state,p,pose,collector.getBuffer(net.minecraft.client.renderer.ItemBlockRenderTypes.getChunkRenderType(state)),true,RandomSource.create(state.getSeed(p)),state.getSeed(p),OverlayTexture.NO_OVERLAY);
                    }else dispatcher.renderBatched(state,p,mc.level,pose,collector.getBuffer(net.minecraft.client.renderer.ItemBlockRenderTypes.getChunkRenderType(state)),true,RandomSource.create(state.getSeed(p)));
                }'''
    else:
        family = 'neoforged.neoforge' if loader == 'neoforge' else 'minecraftforge'
        render = f'''var data=entity==null?net.{family}.client.model.data.ModelData.EMPTY:entity.getModelData();
                data=model.getModelData(mc.level,p,state,data);
                if(state.getRenderShape()==RenderShape.MODEL)for(RenderType type:model.getRenderTypes(state,RandomSource.create(state.getSeed(p)),data))
                    dispatcher.renderBatched(state,p,mc.level,pose,collector.getBuffer(type),true,RandomSource.create(state.getSeed(p)),data,type);'''
    replacements['@RENDER_BLOCK@'] = render
    for old,new in replacements.items():
        runtime=runtime.replace(old,new)
    collector=(ROOT/f'templates/{"neoforge" if loader == "neoforge" else "forge"}-collector.java').read_text()
    write(directory/'src/main/java/dev/msc/mapexport/CaptureClient.java',header+register+runtime+collector)
    shutil.copytree(ROOT/'common',directory/'src/main/java',dirs_exist_ok=True)
    if loader != 'neoforge':
        # Forge's packaged game uses SRG method names; Fabric Loom remaps its
        # annotation/refmap. Both hooks stop only private-world simulation,
        # leaving connection processing alive to deliver restored entity data.
        method='{"tick(Ljava/util/function/BooleanSupplier;)V","m_8793_(Ljava/util/function/BooleanSupplier;)V"}' if loader == 'forge' else '"tick"'
        remap='false' if loader == 'forge' else 'true'
        write(directory/'src/main/java/dev/msc/mapexport/FreezeMixin.java',f'''package dev.msc.mapexport;
@org.spongepowered.asm.mixin.Mixin(net.minecraft.server.level.ServerLevel.class)
public abstract class FreezeMixin {{
 @org.spongepowered.asm.mixin.injection.Inject(method={method},at=@org.spongepowered.asm.mixin.injection.At("HEAD"),cancellable=true,remap={remap},require=1)
 private void mscFreeze(java.util.function.BooleanSupplier time,org.spongepowered.asm.mixin.injection.callback.CallbackInfo ci) {{
  if(FreezeGate.skip(((net.minecraft.server.level.ServerLevel)(Object)this).getServer()))ci.cancel();
 }}
}}
''')
        tick='{"tick(Ljava/util/function/BooleanSupplier;)V","m_104726_(Ljava/util/function/BooleanSupplier;)V"}' if loader == 'forge' else '"tick"'
        entities='{"tickEntities()V","m_104804_()V"}' if loader == 'forge' else '"tickEntities"'
        write(directory/'src/main/java/dev/msc/mapexport/ClientFreezeMixin.java',f'''package dev.msc.mapexport;
@org.spongepowered.asm.mixin.Mixin(net.minecraft.client.multiplayer.ClientLevel.class)
public abstract class ClientFreezeMixin {{
 @org.spongepowered.asm.mixin.injection.Inject(method={tick},at=@org.spongepowered.asm.mixin.injection.At("HEAD"),cancellable=true,remap={remap},require=1)
 private void mscWorldTick(java.util.function.BooleanSupplier time,org.spongepowered.asm.mixin.injection.callback.CallbackInfo ci) {{
  if(FreezeGate.active(net.minecraft.client.Minecraft.getInstance().getSingleplayerServer()))ci.cancel();
 }}
 @org.spongepowered.asm.mixin.injection.Inject(method={entities},at=@org.spongepowered.asm.mixin.injection.At("HEAD"),cancellable=true,remap={remap},require=1)
 private void mscEntityTick(org.spongepowered.asm.mixin.injection.callback.CallbackInfo ci) {{
  if(FreezeGate.active(net.minecraft.client.Minecraft.getInstance().getSingleplayerServer()))ci.cancel();
 }}
}}
''')
        write(directory/'src/main/resources/msc-map-capture.mixins.json',json.dumps({'required':True,'package':'dev.msc.mapexport','compatibilityLevel':'JAVA_17','mixins':['FreezeMixin','ClientFreezeMixin'],**({'refmap':'msc-map-capture.refmap.json'} if loader == 'fabric' else {}),'injectors':{'defaultRequire':1}}))
    java = f'''version = '0.2.0'
group = 'dev.msc.mapexport'
base {{ archivesName = 'msc-map-capture-{loader}' }}
java {{ toolchain.languageVersion = JavaLanguageVersion.of({pin['java']}) }}
tasks.withType(JavaCompile).configureEach {{ options.encoding = 'UTF-8'; options.release = {pin['java']} }}
tasks.withType(AbstractArchiveTask).configureEach {{ preserveFileTimestamps = false; reproducibleFileOrder = true }}
'''
    if loader == 'forge':
        build = f"plugins {{ id 'net.minecraftforge.gradle' version '{pin['plugin']}' }}\n"+java+f"minecraft {{ mappings channel: 'official', version: '{pin['minecraft']}'; accessTransformer = file('src/main/resources/META-INF/accesstransformer.cfg') }}\ndependencies {{ minecraft 'net.minecraftforge:forge:{pin['minecraft']}-{pin['loader']}' }}\ntasks.named('jar').configure {{ finalizedBy 'reobfJar'; manifest.attributes('MixinConfigs':'msc-map-capture.mixins.json') }}\n"
        shutil.copyfile(ROOT.parent/'forge-1.20.1/src/main/resources/META-INF/accesstransformer.cfg',directory/'src/main/resources/META-INF/accesstransformer.cfg') if (directory/'src/main/resources/META-INF').exists() else write(directory/'src/main/resources/META-INF/accesstransformer.cfg',(ROOT.parent/'forge-1.20.1/src/main/resources/META-INF/accesstransformer.cfg').read_text())
        repository = 'https://maven.minecraftforge.net'
    elif loader == 'neoforge':
        build = f"plugins {{ id 'net.neoforged.moddev' version '{pin['plugin']}' }}\n"+java+f"neoForge {{ version = '{pin['loader']}'; accessTransformers.from(file('src/main/resources/META-INF/accesstransformer.cfg')) }}\n"
        at=(ROOT.parents[1]/'world-map-proof/client-capture/src/main/resources/META-INF/accesstransformer.cfg').read_text()+ '\npublic net.minecraft.client.renderer.RenderStateShard name\n'
        write(directory/'src/main/resources/META-INF/accesstransformer.cfg',at)
        repository = 'https://maven.neoforged.net/releases'
    else:
        build = f"plugins {{ id 'fabric-loom' version '{pin['plugin']}' }}\n"+java+f"repositories {{ mavenCentral() }}\nloom {{ accessWidenerPath = file('src/main/resources/msc-map-capture.accesswidener'); mixin.defaultRefmapName = 'msc-map-capture.refmap.json' }}\ndependencies {{ minecraft 'com.mojang:minecraft:{pin['minecraft']}'; mappings loom.officialMojangMappings(); modImplementation 'net.fabricmc:fabric-loader:{pin['loader']}'; modImplementation 'net.fabricmc.fabric-api:fabric-api:{pin['api']}' }}\n"
        aw='''accessWidener v2 named
accessible class net/minecraft/client/renderer/RenderType$CompositeRenderType
accessible class net/minecraft/client/renderer/RenderType$CompositeState
accessible class net/minecraft/client/renderer/RenderStateShard$EmptyTextureStateShard
accessible field net/minecraft/client/renderer/RenderStateShard name Ljava/lang/String;
accessible method net/minecraft/client/renderer/RenderType$CompositeRenderType state ()Lnet/minecraft/client/renderer/RenderType$CompositeState;
accessible field net/minecraft/client/renderer/RenderType$CompositeState textureState Lnet/minecraft/client/renderer/RenderStateShard$EmptyTextureStateShard;
accessible method net/minecraft/client/renderer/RenderStateShard$EmptyTextureStateShard cutoutTexture ()Ljava/util/Optional;
accessible field net/minecraft/client/renderer/LightTexture lightPixels Lcom/mojang/blaze3d/platform/NativeImage;
accessible field com/mojang/blaze3d/systems/RenderSystem shaderLightDirections [Lorg/joml/Vector3f;
'''
        write(directory/'src/main/resources/msc-map-capture.accesswidener',aw)
        write(directory/'src/main/resources/fabric.mod.json',json.dumps({'schemaVersion':1,'id':'msc_map_capture','version':'0.2.0','name':'MSC Map Capture','environment':'client','license':'Apache-2.0','entrypoints':{'client':['dev.msc.mapexport.CaptureClient']},'mixins':['msc-map-capture.mixins.json'],'accessWidener':'msc-map-capture.accesswidener','depends':{'minecraft':pin['minecraft'],'fabricloader':pin['loader'],'fabric-api':pin['api'],'java':f'>={pin["java"]}'}}))
        repository='https://maven.fabricmc.net'
    if loader != 'fabric':
        dependency='neoforge' if loader == 'neoforge' else 'forge'
        meta=f'''modLoader="javafml"
loaderVersion="[{4 if loader == 'neoforge' else 47},)"
license="Apache-2.0"
[[mods]]
modId="msc_map_capture"
version="0.2.0"
displayName="MSC Map Capture"
description="Explicit bounded capture inside a dedicated private client."
[[dependencies.msc_map_capture]]
modId="{dependency}"
{ 'type="required"' if loader == 'neoforge' else 'mandatory=true' }
versionRange="[{pin['loader']}]"
side="CLIENT"
[[dependencies.msc_map_capture]]
modId="minecraft"
{ 'type="required"' if loader == 'neoforge' else 'mandatory=true' }
versionRange="[{pin['minecraft']}]"
side="CLIENT"
'''
        write(directory/f'src/main/resources/META-INF/{"neoforge.mods.toml" if loader == "neoforge" else "mods.toml"}',meta)
    write(directory/'build.gradle',build)
    write(directory/'settings.gradle',f"pluginManagement {{ repositories {{ maven {{ url = '{repository}' }}; gradlePluginPortal(); mavenCentral() }} }}\nrootProject.name = 'msc-map-capture-{loader}'\n")
    write(directory/'gradle.properties','org.gradle.jvmargs=-Xmx2G\norg.gradle.daemon=false\norg.gradle.parallel=false\n')


def source_digest():
    digest=hashlib.sha256()
    paths=[path for path in ROOT.rglob('*') if path.is_file() and '__pycache__' not in path.parts]
    # Generation also consumes preserved proof imports/access rules. Changes
    # there must invalidate an otherwise source-current packaged helper set.
    paths += [
        ROOT.parent/'forge-1.20.1/src/main/java/dev/msc/mapexport/capture/CaptureFixture.java',
        ROOT.parent/'forge-1.20.1/src/main/resources/META-INF/accesstransformer.cfg',
        ROOT.parents[1]/'world-map-proof/client-capture/src/main/java/dev/msc/mapproof/capture/CaptureProof.java',
        ROOT.parents[1]/'world-map-proof/client-capture/src/main/resources/META-INF/accesstransformer.cfg',
        ROOT.parents[2]/'LICENSE',
    ]
    for path in sorted(paths):
        digest.update(path.relative_to(ROOT.parents[2]).as_posix().encode());digest.update(b'\0');digest.update(path.read_bytes());digest.update(b'\0')
    return digest.hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,default=ROOT.parents[2]/'target/map-capture-helpers')
    parser.add_argument('--loader',choices=list(PINS),action='append')
    for major in (17,21):parser.add_argument(f'--java{major}-home')
    for version in ('8.8','9.2.1'):parser.add_argument('--gradle'+version.replace('.','-'))
    args=parser.parse_args();output=args.output.resolve();output.mkdir(parents=True,exist_ok=True)
    build_source=source_digest()
    descriptors=[]
    for loader in args.loader or PINS:
        pin=PINS[loader];home=java_home(pin['java'],getattr(args,f'java{pin["java"]}_home'));executable=gradle(pin['gradle'],getattr(args,'gradle'+pin['gradle'].replace('.','_')))
        environment=dict(os.environ,JAVA_HOME=home);environment['PATH']=str(Path(home)/'bin')+os.pathsep+environment.get('PATH','')
        # Inspect exact Gradle version with the selected JDK before any build.
        result=subprocess.run([executable,'--version'],env=environment,capture_output=True,text=True,check=True)
        if f"Gradle {pin['gradle']}\n" not in result.stdout:raise ValueError(f"Expected Gradle {pin['gradle']}; received a different toolchain")
        directory=output/'projects'/loader;directory.mkdir(parents=True,exist_ok=True);generate(loader,directory)
        tasks=['jar','reobfJar'] if loader=='forge' else ['remapJar'] if loader=='fabric' else ['jar']
        subprocess.run([executable,'--no-daemon','--max-workers=2','--console=plain',*tasks],cwd=directory,env=environment,check=True)
        jar=directory/f'build/libs/msc-map-capture-{loader}-0.2.0.jar';raw=jar.read_bytes();target=output/jar.name;target.write_bytes(raw)
        descriptors.append(dict(pin,loaderFamily=loader,file=target.name,bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest(),format='msc-contextual-mesh-1',buildOnly=True,gameLaunched=False))
    # Publish only a complete selected target set, after all requested builds pass.
    if source_digest()!=build_source:raise ValueError('Helper source changed during the build; no manifest published')
    write(output/'helpers.json',json.dumps({'version':1,'buildSourceSha256':build_source,'helpers':descriptors},indent=2)+'\n')
    shutil.copyfile(ROOT.parents[2]/'LICENSE',output/'LICENSE')
    shutil.copyfile(ROOT/'DEPENDENCIES.md',output/'DEPENDENCIES.md')
    print(f'Built {len(descriptors)} adapters: {output / "helpers.json"}')

if __name__=='__main__':
    try:main()
    except (ValueError,subprocess.CalledProcessError) as error:
        print(f'Map capture build failed: {error}',file=sys.stderr);sys.exit(1)
