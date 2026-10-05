import * as THREE from 'three';
import { OrbitControls } from '/vendor/OrbitControls.js';

const canvas = document.querySelector('#scene');
const status = document.querySelector('#status');
const scene = new THREE.Scene();
scene.background = new THREE.Color('#0d0d0f');
const camera = new THREE.PerspectiveCamera(55,1,0.01,1000);
const renderer = new THREE.WebGLRenderer({ canvas, antialias:true, preserveDrawingBuffer:true });
renderer.setPixelRatio(Math.min(devicePixelRatio,2));
const controls = new OrbitControls(camera,canvas);
controls.enableDamping = false;
let visible = new THREE.Group();
let current;
let serial = 0;
scene.add(visible);
function home() {camera.position.set(13,72,14);controls.target.set(6,65.5,3);controls.update();}
home();
new ResizeObserver(() => {
  const bounds=canvas.getBoundingClientRect();
  renderer.setSize(bounds.width,bounds.height,false);
  camera.aspect=bounds.width/bounds.height;camera.updateProjectionMatrix();
}).observe(canvas);
renderer.setAnimationLoop(()=>renderer.render(scene,camera));

document.querySelector('#home').onclick=home;
document.querySelector('#image').onclick=()=>{
  renderer.render(scene,camera);
  const link=document.createElement('a');link.href=canvas.toDataURL('image/png');link.download='msc-client-mesh-saved-frame.png';link.click();
};
const vertexShader=`uniform float directional; uniform vec3 light0; uniform vec3 light1; attribute vec4 captureColor; varying vec2 captureUv; varying vec4 savedColor;
void main(){captureUv=uv;savedColor=captureColor;float light=min(1.0,(max(0.0,dot(light0,normal))+max(0.0,dot(light1,normal)))*0.6+0.4);savedColor.rgb*=mix(1.0,light,directional);gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`;
const fragmentShader=`uniform sampler2D capturedTexture;uniform float threshold;varying vec2 captureUv;varying vec4 savedColor;
void main(){vec4 pixel=texture2D(capturedTexture,captureUv)*savedColor;if(pixel.a<threshold)discard;gl_FragColor=pixel;}`;
function dispose(group){
  const textures=new Set();
  group.traverse(item=>{if(item.geometry)item.geometry.dispose();if(item.material){textures.add(item.material.uniforms.capturedTexture.value);item.material.dispose();}});
  for(const texture of textures)texture.dispose();
}
async function response(url){const r=await fetch(url,{cache:'no-store'});if(!r.ok)throw new Error(await r.text());return r;}
async function load(){
  const request=++serial;
  const candidate=new THREE.Group();
  const textures=[];
  try{
    status.textContent='Validating the published capture…';
    const input=await (await response('/api/current')).json();
    const {manifest,report,id}=input;
    const loader=new THREE.TextureLoader();
    const cache=new Map();
    for(const material of manifest.materials){
      if(!cache.has(material.texture)){
        const texture=await loader.loadAsync(`/capture/${id}/${material.texture}`);
        texture.flipY=false;texture.colorSpace=THREE.NoColorSpace;
        texture.minFilter=THREE.NearestFilter;texture.magFilter=THREE.NearestFilter;
        texture.generateMipmaps=false;cache.set(material.texture,texture);textures.push(texture);
      }
    }
    for(const item of manifest.meshes){
      const data=await (await response(`/capture/${id}/${item.file}`)).json();
      const saved=manifest.materials[item.material];
      const geometry=new THREE.BufferGeometry();
      geometry.setAttribute('position',new THREE.Float32BufferAttribute(data.positions,3));
      geometry.setAttribute('uv',new THREE.Float32BufferAttribute(data.uv,2));
      geometry.setAttribute('normal',new THREE.Float32BufferAttribute(data.normals,3));
      geometry.setAttribute('captureColor',new THREE.Float32BufferAttribute(data.colors,4));
      geometry.setIndex(data.indices);
      const material=new THREE.ShaderMaterial({vertexShader,fragmentShader,uniforms:{capturedTexture:{value:cache.get(saved.texture)},threshold:{value:saved.alphaThreshold},directional:{value:saved.directionalLighting?1:0},light0:{value:new THREE.Vector3(...manifest.directionalLights[0])},light1:{value:new THREE.Vector3(...manifest.directionalLights[1])}},
        side:saved.cull?THREE.FrontSide:THREE.DoubleSide,transparent:saved.mode==='translucent',depthWrite:saved.depthWrite,toneMapped:false});
      candidate.add(new THREE.Mesh(geometry,material));
    }
    if(request!==serial){dispose(candidate);return;}
    scene.remove(visible);dispose(visible);visible=candidate;scene.add(visible);current=input;
    status.textContent=`Saved frame · tick ${manifest.gameTick}, partial tick 0 · ${report.vertices} vertices · visual acceptance pending`;
    document.querySelector('#receipt').textContent=`Snapshot ${manifest.snapshotSha256.slice(0,16)}… / context ${manifest.contextSha256.slice(0,16)}… / ${manifest.capturedAt}`;
    document.querySelector('#details').textContent=JSON.stringify({scope:report.scope,materials:manifest.materials,refusals:report.refusals,objects:manifest.objects.filter(o=>o.role!=='surroundings')},null,2);
    for(const id of ['custom','context','frame'])document.querySelector(`#${id}`).checked=false;
    document.querySelector('#confirmation').textContent='Visual acceptance remains pending.';
    ready();
  }catch(error){dispose(candidate);for(const texture of textures)texture.dispose();if(request===serial)status.textContent=`Capture not adopted: ${error.message}`;}
}
document.querySelector('#load').onclick=load;
function ready(){document.querySelector('#confirm').disabled=!current||!['custom','context','frame'].every(id=>document.querySelector(`#${id}`).checked);}
for(const id of ['custom','context','frame'])document.querySelector(`#${id}`).onchange=ready;
document.querySelector('#confirm').onclick=async()=>{
  try{
    renderer.render(scene,camera);
    const r=await fetch('/api/evidence',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({captureSha256:current.report.captureSha256,
      confirmations:{customLoader:true,contextPair:true,savedFrame:true},image:canvas.toDataURL('image/png').split(',')[1]})});
    if(!r.ok)throw new Error(await r.text());
    document.querySelector('#confirmation').textContent='Owner-confirmed evidence saved in the private workspace. This confirms this fixture only.';
  }catch(error){document.querySelector('#confirmation').textContent=error.message;}
};
