import * as THREE from 'three';
import type { VantageViewer } from '@thoughts-on-things/vantage-mc/three';
import type { Schema } from '../shared/types';

type Manifest = Schema['MapCaptureManifestDTO'];
type MeshData = Schema['MapCaptureMeshDataDTO'];
export type CaptureReference = { format: string; path: string; sha256: string; digest: string };
export type CapturedAppearance = { dispose(): void; label: string };

const vertexShader = `
uniform float directional; uniform vec3 light0; uniform vec3 light1;
attribute vec4 captureColor; varying vec2 captureUv; varying vec4 savedColor; varying float worldY;
void main(){captureUv=uv;worldY=position.y;savedColor=captureColor;
float light=min(1.0,(max(0.0,dot(light0,normal))+max(0.0,dot(light1,normal)))*0.6+0.4);
savedColor.rgb*=mix(1.0,light,directional);gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`;
const fragmentShader = `
uniform sampler2D capturedTexture;uniform float threshold;uniform float clipY;
varying vec2 captureUv;varying vec4 savedColor;varying float worldY;
void main(){if(worldY>clipY)discard;vec4 pixel=texture2D(capturedTexture,captureUv)*savedColor;
if(pixel.a<threshold)discard;gl_FragColor=pixel;}`;

async function sha256(raw: ArrayBuffer): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', raw);
  return Array.from(new Uint8Array(hash), (n) => n.toString(16).padStart(2, '0')).join('');
}

/** Candidate-only loading: any failure disposes all objects before scene adoption. */
export async function loadCapturedAppearance(
  viewer: VantageViewer,
  reference: CaptureReference,
  read: (path: string) => Promise<ArrayBuffer>,
  binding: Schema['MapAssetsBindingDTO'] | undefined,
  resourceGeneration: string | undefined,
  cancelled: () => boolean,
): Promise<CapturedAppearance> {
  const group = new THREE.Group();
  const geometries: THREE.BufferGeometry[] = [];
  const materials: THREE.ShaderMaterial[] = [];
  const textures = new Map<string, THREE.Texture>();
  const images: ImageBitmap[] = [];
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    viewer.scene.remove(group);
    for (const geometry of geometries) geometry.dispose();
    for (const material of materials) material.dispose();
    for (const texture of textures.values()) texture.dispose();
    for (const image of images) image.close();
  };
  const check = () => {
    if (cancelled()) throw new DOMException('Cancelled', 'AbortError');
  };
  try {
    if (reference.format !== 'msc-contextual-mesh-1' || reference.path !== 'capture/capture.json')
      throw new Error('Unsupported captured appearance format. The existing map is retained.');
    const raw = await read(reference.path);
    check();
    if (raw.byteLength > 8 * 1024 * 1024 || (await sha256(raw)) !== reference.sha256)
      throw new Error('Captured appearance manifest failed its checksum.');
    const manifest: Manifest = JSON.parse(new TextDecoder().decode(raw));
    if (
      !binding ||
      !manifest.savedFrame ||
      manifest.request.format !== reference.format ||
      manifest.observedSnapshotId !== manifest.request.snapshotId ||
      manifest.observedInputFingerprint !== manifest.request.inputFingerprint ||
      manifest.request.binding.agentHostId !== binding.agentHostId ||
      manifest.request.binding.serverId !== binding.serverId ||
      manifest.request.binding.slotId !== binding.slotId ||
      manifest.request.binding.worldIncarnation !== binding.worldIncarnation ||
      manifest.request.binding.revision !== binding.revision ||
      manifest.request.resourceGenerationId !== resourceGeneration ||
      manifest.blocks.length > 4096 ||
      manifest.materials.length > 1024 ||
      Object.keys(manifest.files).length > 4096
    )
      throw new Error('Captured appearance belongs to different saved map inputs.');
    let total = 0;
    const checked = async (path: string): Promise<ArrayBuffer> => {
      if (
        !/^(meshes\/[^\\:]+\.json|textures\/[^\\:]+\.png)$/.test(path) ||
        path.split('/').some((p) => !p || p === '.' || p === '..')
      )
        throw new Error('Unsafe captured appearance path.');
      const file = manifest.files[path];
      if (!file || file.bytes <= 0 || file.bytes > 16 * 1024 * 1024)
        throw new Error('Captured appearance file exceeds its bounds.');
      total += file.bytes;
      if (total > 64 * 1024 * 1024)
        throw new Error('Captured appearance exceeds its memory budget.');
      const bytes = await read(`capture/${path}`);
      check();
      if (bytes.byteLength !== file.bytes || (await sha256(bytes)) !== file.sha256)
        throw new Error('Captured appearance file failed its checksum.');
      return bytes;
    };
    let decodedTextures = 0;
    for (const material of manifest.materials) {
      if (textures.has(material.texture)) continue;
      const bytes = await checked(material.texture);
      const image = await createImageBitmap(new Blob([bytes], { type: 'image/png' }), {
        premultiplyAlpha: 'none',
        colorSpaceConversion: 'none',
        imageOrientation: 'none',
      });
      images.push(image);
      check();
      if (image.width > 4096 || image.height > 4096)
        throw new Error('Captured texture exceeds its bounds.');
      decodedTextures += image.width * image.height * 4;
      if (decodedTextures > 128 * 1024 * 1024)
        throw new Error('Captured textures exceed their decoded memory budget.');
      const texture = new THREE.Texture(image);
      texture.flipY = false;
      texture.colorSpace = THREE.NoColorSpace;
      texture.minFilter = THREE.NearestFilter;
      texture.magFilter = THREE.NearestFilter;
      texture.generateMipmaps = false;
      texture.needsUpdate = true;
      textures.set(material.texture, texture);
      viewer.renderer.initTexture(texture);
    }
    let vertices = 0;
    let indices = 0;
    // Share the terrain's animated cutaway plane once a streaming tile exists.
    let clip: { value: number } | undefined;
    for (const block of manifest.blocks) {
      for (const item of block.meshes) {
        const data: MeshData = JSON.parse(new TextDecoder().decode(await checked(item.file)));
        const n = data.positions.length / 3;
        vertices += n;
        indices += data.indices.length;
        if (
          !Number.isInteger(n) ||
          n <= 0 ||
          vertices > 1_000_000 ||
          indices > 6_000_000 ||
          data.normals.length !== n * 3 ||
          data.uv.length !== n * 2 ||
          data.colors.length !== n * 4 ||
          !data.indices.length ||
          data.indices.length % 3 ||
          data.indices.some((i) => !Number.isInteger(i) || i < 0 || i >= n) ||
          !data.positions.every(Number.isFinite) ||
          !data.normals.every(Number.isFinite) ||
          !data.uv.every(Number.isFinite) ||
          !data.colors.every(Number.isFinite)
        )
          throw new Error('Invalid captured appearance geometry.');
        const saved = manifest.materials[item.material];
        if (!saved || !['opaque', 'cutout', 'translucent', 'additive'].includes(saved.mode))
          throw new Error('Unsupported captured appearance material.');
        const geometry = new THREE.BufferGeometry();
        geometries.push(geometry);
        geometry.setAttribute('position', new THREE.Float32BufferAttribute(data.positions, 3));
        geometry.setAttribute('normal', new THREE.Float32BufferAttribute(data.normals, 3));
        geometry.setAttribute('uv', new THREE.Float32BufferAttribute(data.uv, 2));
        geometry.setAttribute('captureColor', new THREE.Float32BufferAttribute(data.colors, 4));
        geometry.setIndex(data.indices);
        geometry.computeBoundingSphere();
        const material = new THREE.ShaderMaterial({
          vertexShader,
          fragmentShader,
          uniforms: {
            capturedTexture: { value: textures.get(saved.texture) },
            threshold: { value: saved.alphaThreshold },
            directional: { value: saved.directionalLighting ? 1 : 0 },
            light0: { value: new THREE.Vector3(...manifest.directionalLights[0]) },
            light1: { value: new THREE.Vector3(...manifest.directionalLights[1]) },
            clipY: { value: viewer.slice ?? 1e9 },
          },
          side: saved.cull ? THREE.FrontSide : THREE.DoubleSide,
          transparent: saved.mode === 'translucent' || saved.mode === 'additive',
          blending: saved.mode === 'additive' ? THREE.AdditiveBlending : THREE.NormalBlending,
          depthWrite: saved.depthWrite,
          toneMapped: false,
        });
        materials.push(material);
        const mesh = new THREE.Mesh(geometry, material);
        mesh.onBeforeRender = () => {
          if (!clip)
            viewer.scene.traverse((object) => {
              if (
                object instanceof THREE.Mesh &&
                object.material instanceof THREE.ShaderMaterial &&
                object.material.uniforms.uClipY
              )
                clip = object.material.uniforms.uClipY;
            });
          material.uniforms.clipY.value = clip?.value ?? viewer.slice ?? 1e9;
        };
        group.add(mesh);
      }
    }
    check();
    viewer.scene.add(group);
    const previousShaderError = viewer.renderer.debug.onShaderError;
    const previousCheck = viewer.renderer.debug.checkShaderErrors;
    let shaderFailed = false;
    viewer.renderer.debug.checkShaderErrors = true;
    viewer.renderer.debug.onShaderError = (...args) => {
      shaderFailed = true;
      previousShaderError?.(...args);
    };
    try {
      await viewer.renderer.compileAsync(group, viewer.camera, viewer.scene);
      check();
      // Shader checks can be deferred until uniforms are first used. Draw the
      // hidden candidate before disposing the user's current viewer.
      viewer.renderer.render(viewer.scene, viewer.camera);
      if (shaderFailed)
        throw new Error('Captured materials could not render. The existing map is retained.');
    } finally {
      viewer.renderer.debug.onShaderError = previousShaderError;
      viewer.renderer.debug.checkShaderErrors = previousCheck;
    }
    check();
    return {
      dispose,
      label: `Captured saved frame · ${new Date(manifest.capturedAtUnix * 1000).toLocaleString()} · tick ${manifest.gameTick}`,
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
