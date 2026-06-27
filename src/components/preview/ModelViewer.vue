<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { OBJLoader } from "three/examples/jsm/loaders/OBJLoader.js";
import { FBXLoader } from "three/examples/jsm/loaders/FBXLoader.js";
import { ColladaLoader } from "three/examples/jsm/loaders/ColladaLoader.js";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileNode } from "../../types/library";

const { t } = useI18n();
const props = defineProps<{ file: FileNode }>();
const containerRef = ref<HTMLElement | null>(null);
const status = ref(t("preview.loadingModel"));
const wireframe = ref(false);

let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let controls: OrbitControls | null = null;
let frameId = 0;
let currentObjects: THREE.Object3D[] = [];

function clearScene() {
  for (const o of currentObjects) scene?.remove(o);
  currentObjects = [];
}

function setupScene() {
  const el = containerRef.value!;
  const w = el.clientWidth || 400;
  const h = el.clientHeight || 300;
  scene = new THREE.Scene();
  scene.background = new THREE.Color(0x0f172a);
  camera = new THREE.PerspectiveCamera(50, w / h, 0.01, 1000);
  camera.position.set(3, 2, 5);
  renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setSize(w, h);
  renderer.setPixelRatio(window.devicePixelRatio);
  el.appendChild(renderer.domElement);
  controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  scene.add(new THREE.HemisphereLight(0xffffff, 0x444444, 1.2));
  const dir = new THREE.DirectionalLight(0xffffff, 1.0);
  dir.position.set(5, 10, 7);
  scene.add(dir);
  animate();
}

function animate() {
  frameId = requestAnimationFrame(animate);
  controls?.update();
  if (renderer && scene && camera) renderer.render(scene, camera);
}

function fitCameraToObject(object: THREE.Object3D) {
  const box = new THREE.Box3().setFromObject(object);
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z, 0.001);
  const fov = (camera!.fov * Math.PI) / 180;
  let dist = maxDim / 2 / Math.tan(fov / 2);
  dist *= 1.8;
  camera!.position.set(center.x + dist, center.y + dist * 0.7, center.z + dist);
  camera!.lookAt(center);
  controls!.target.copy(center);
  controls!.update();
}

function addObject(obj: THREE.Object3D) {
  // 默认材质（obj 无材质时）
  obj.traverse((child) => {
    const mesh = child as THREE.Mesh;
    if (mesh.isMesh && !mesh.material) {
      mesh.material = new THREE.MeshStandardMaterial({ color: 0x88aacc });
    }
  });
  scene!.add(obj);
  currentObjects.push(obj);
  fitCameraToObject(obj);
  status.value = "";
}

async function loadModel(f: FileNode) {
  clearScene();
  wireframe.value = false;
  status.value = t("preview.loadingModel");
  const url = convertFileSrc(f.abs_path);
  const ext = f.ext.toLowerCase();
  try {
    if (ext === "glb" || ext === "gltf") {
      const gltf = await loadAny(new GLTFLoader() as any, url);
      addObject((gltf as any).scene);
    } else if (ext === "obj") {
      const obj = await loadAny(new OBJLoader() as any, url);
      addObject(obj as THREE.Object3D);
    } else if (ext === "dae") {
      const res = await loadAny(new ColladaLoader() as any, url);
      addObject((res as any).scene);
    } else if (ext === "fbx") {
      try {
        const obj = await loadAny(new FBXLoader() as any, url);
        addObject(obj as THREE.Object3D);
      } catch (fbxErr) {
        const msg = String(fbxErr);
        if (msg.includes("not supported") || msg.includes("version")) {
          status.value = t("preview.fbxVersionError");
        } else {
          throw fbxErr;
        }
      }
    } else if (ext === "blend") {
      status.value = t("preview.convertingGlb");
      const { getModelGlb } = await import("../../ipc/model");
      const res = await getModelGlb(f.abs_path);
      if (res.source === "error" || !res.path) {
        status.value = res.message;
      } else {
        status.value = t("preview.loadingModel");
        const gltf = await loadAny(new GLTFLoader() as any, convertFileSrc(res.path));
        addObject((gltf as any).scene);
      }
    } else if (ext === "mtl" || ext === "dds" || ext === "tga") {
      status.value = t("preview.auxResource", { ext });
    } else {
      status.value = t("preview.unsupported3d") + ` (${ext})`;
    }
  } catch (e) {
    status.value = t("preview.loadFailed", { msg: String(e) });
  }
}

function loadAny(loader: any, url: string): Promise<unknown> {
  return new Promise((resolve, reject) => {
    loader.load(url, resolve, undefined, reject);
  });
}

function toggleWireframe() {
  wireframe.value = !wireframe.value;
  for (const o of currentObjects) {
    o.traverse((child) => {
      const mesh = child as THREE.Mesh;
      if (mesh.isMesh && mesh.material) {
        const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
        for (const m of mats) {
          (m as any).wireframe = wireframe.value;
        }
      }
    });
  }
}

onMounted(() => {
  setupScene();
  loadModel(props.file);
});

watch(() => props.file, (f) => loadModel(f));

onUnmounted(() => {
  cancelAnimationFrame(frameId);
  renderer?.dispose();
  if (renderer && containerRef.value && renderer.domElement.parentNode === containerRef.value) {
    containerRef.value.removeChild(renderer.domElement);
  }
});
</script>

<template>
  <div class="w-full h-full flex flex-col">
    <div class="flex items-center gap-2 px-2 py-1 bg-slate-800 border-b border-slate-700 text-xs shrink-0">
      <button class="px-2 py-0.5 rounded bg-slate-600 hover:bg-slate-500" @click="toggleWireframe">
        {{ wireframe ? t("preview.solid") : t("preview.wireframe") }}
      </button>
      <span class="text-slate-500">{{ t("preview.modelRotateHint") }}</span>
    </div>
    <div ref="containerRef" class="flex-1 relative">
      <div
        v-if="status"
        class="absolute inset-0 flex items-center justify-center text-xs text-slate-400 pointer-events-none text-center px-4"
      >
        {{ status }}
      </div>
    </div>
  </div>
</template>
