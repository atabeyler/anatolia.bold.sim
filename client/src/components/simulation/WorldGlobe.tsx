import { useRef, useMemo, useState, useEffect, memo } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { OrbitControls, Stars } from '@react-three/drei';
import * as THREE from 'three';
import { useSimStore, type CentroidPoint } from '../../store/simStore';

// Self-hosted (was raw.githubusercontent.com) -- the app runs under
// Cross-Origin-Embedder-Policy: require-corp (see vite.config.ts / main.rs)
// for maximum cross-browser SharedArrayBuffer/thread support, and GitHub's
// raw content CDN sends no Cross-Origin-Resource-Policy header, so a
// cross-origin <img>/texture load would be blocked outright under
// require-corp. Source: mrdoob/three.js r128 examples/textures/planets.
const EARTH_MAP  = '/textures/earth_atmos_2048.jpg';
const EARTH_BUMP = '/textures/earth_normal_2048.jpg';
const EARTH_SPEC = '/textures/earth_specular_2048.jpg';

/** Build a round soft-glow sprite texture via the Canvas 2D API. */
function makeSpriteTexture(): THREE.CanvasTexture {
  const size = 128;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;
  const half = size / 2;
  const gradient = ctx.createRadialGradient(half, half, half * 0.08, half, half, half * 0.55);
  gradient.addColorStop(0, 'rgba(255,255,255,1)');
  gradient.addColorStop(0.72, 'rgba(255,255,255,1)');
  gradient.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, size, size);
  const tex = new THREE.CanvasTexture(canvas);
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.needsUpdate = true;
  return tex;
}

let sharedSpriteTexture: THREE.CanvasTexture | null = null;
function getSharedSpriteTexture(): THREE.CanvasTexture {
  if (!sharedSpriteTexture) {
    sharedSpriteTexture = makeSpriteTexture();
    sharedSpriteTexture.needsUpdate = true;
    // Prevent R3F auto-disposal from invalidating this shared texture when
    // any one of the materials that reference it gets unmounted.
    sharedSpriteTexture.dispose = () => {};
  }
  return sharedSpriteTexture;
}

/** Jupiter: gradient bands + Great Red Spot */
function makeJupiterTex(): THREE.CanvasTexture {
  const W = 1024, H = 512;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const bd: [number,string][] = [
    [0.00,'#c07030'],[0.06,'#f0d090'],[0.12,'#b85820'],[0.18,'#e8c880'],
    [0.25,'#d07838'],[0.32,'#f4e09a'],[0.40,'#c06828'],[0.48,'#f0cc80'],
    [0.54,'#c88040'],[0.60,'#e8c078'],[0.66,'#b86020'],[0.72,'#ecc880'],
    [0.78,'#c07830'],[0.86,'#f0d090'],[0.93,'#c06030'],[1.00,'#c07030'],
  ];
  for (let i = 0; i < bd.length - 1; i++) {
    const y1 = bd[i][0] * H, y2 = bd[i+1][0] * H;
    const g = ctx.createLinearGradient(0,y1,0,y2);
    g.addColorStop(0,bd[i][1]); g.addColorStop(1,bd[i+1][1]);
    ctx.fillStyle = g; ctx.fillRect(0,y1,W,y2-y1+1);
  }
  ctx.globalAlpha = 0.14;
  for (let y = 0; y < H; y += 3) {
    const wave = Math.sin(y*0.06)*14 + Math.sin(y*0.025)*20;
    const dark = ((y/H*bd.length)|0) % 2 === 0;
    for (let x = 0; x < W; x += 3)
      if (Math.sin((x+wave)*0.035+y*0.015) > 0.62) {
        ctx.fillStyle = dark ? 'rgba(70,30,5,1)' : 'rgba(250,210,130,1)';
        ctx.fillRect(x,y,3,3);
      }
  }
  ctx.globalAlpha = 1;
  ctx.save(); ctx.translate(W*0.68,H*0.575); ctx.scale(1,0.5);
  const grs = ctx.createRadialGradient(0,0,0,0,0,W*0.056);
  grs.addColorStop(0,'rgba(175,50,15,0.97)'); grs.addColorStop(0.35,'rgba(185,65,22,0.83)');
  grs.addColorStop(0.68,'rgba(160,68,28,0.55)'); grs.addColorStop(1,'rgba(0,0,0,0)');
  ctx.fillStyle = grs; ctx.beginPath(); ctx.arc(0,0,W*0.056,0,Math.PI*2); ctx.fill();
  ctx.restore();
  return new THREE.CanvasTexture(canvas);
}

/** Saturn: warm golden bands */
function makeSaturnTex(): THREE.CanvasTexture {
  const W = 1024, H = 512;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const bd: [number,string][] = [
    [0.00,'#d4b870'],[0.10,'#e8cc88'],[0.22,'#c8a858'],[0.34,'#ead8a0'],
    [0.46,'#d0b868'],[0.56,'#ecdca8'],[0.66,'#c8a858'],[0.76,'#ead8a0'],
    [0.86,'#d4b870'],[1.00,'#d4b870'],
  ];
  for (let i = 0; i < bd.length - 1; i++) {
    const y1 = bd[i][0]*H, y2 = bd[i+1][0]*H;
    const g = ctx.createLinearGradient(0,y1,0,y2);
    g.addColorStop(0,bd[i][1]); g.addColorStop(1,bd[i+1][1]);
    ctx.fillStyle = g; ctx.fillRect(0,y1,W,y2-y1+1);
  }
  ctx.globalAlpha = 0.08;
  for (let y = 0; y < H; y += 4) {
    const wave = Math.sin(y*0.04)*10;
    for (let x = 0; x < W; x += 4)
      if (Math.sin((x+wave)*0.03+y*0.01) > 0.6) {
        ctx.fillStyle = ((y/H*9)|0)%2===0 ? 'rgba(100,70,20,1)' : 'rgba(255,235,165,1)';
        ctx.fillRect(x,y,4,4);
      }
  }
  ctx.globalAlpha = 1;
  return new THREE.CanvasTexture(canvas);
}

/** Saturn ring: C / B / Cassini-gap / A gradient */
function makeSaturnRingTex(): THREE.CanvasTexture {
  const W = 512, H = 1;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const g = ctx.createLinearGradient(0,0,W,0);
  g.addColorStop(0,    'rgba(0,0,0,0)');
  g.addColorStop(0.05, 'rgba(160,140,100,0.15)');
  g.addColorStop(0.20, 'rgba(200,180,130,0.55)');
  g.addColorStop(0.42, 'rgba(235,215,165,0.88)');
  g.addColorStop(0.56, 'rgba(210,190,140,0.72)');
  g.addColorStop(0.60, 'rgba(12,6,2,0.08)');
  g.addColorStop(0.63, 'rgba(12,6,2,0.06)');
  g.addColorStop(0.68, 'rgba(190,170,120,0.60)');
  g.addColorStop(0.82, 'rgba(175,155,108,0.48)');
  g.addColorStop(0.90, 'rgba(155,138,98,0.28)');
  g.addColorStop(1.00, 'rgba(0,0,0,0)');
  ctx.fillStyle = g; ctx.fillRect(0,0,W,H);
  return new THREE.CanvasTexture(canvas);
}

/** Mars: red terrain + polar ice caps */
function makeMarsTex(): THREE.CanvasTexture {
  const W = 1024, H = 512;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const base = ctx.createLinearGradient(0,0,0,H);
  base.addColorStop(0,   '#e0e0d8'); base.addColorStop(0.07,'#cc5030');
  base.addColorStop(0.28,'#c84c2a'); base.addColorStop(0.50,'#d05c38');
  base.addColorStop(0.58,'#b03c24'); base.addColorStop(0.72,'#cc5030');
  base.addColorStop(0.90,'#c04428'); base.addColorStop(0.95,'#e0e0d8');
  base.addColorStop(1,   '#f0f0e8');
  ctx.fillStyle = base; ctx.fillRect(0,0,W,H);
  const patches: [number,number,number,number,string][] = [
    [0.35,0.40,0.12,0.09,'rgba(90,28,8,0.38)'],
    [0.62,0.52,0.26,0.06,'rgba(80,22,6,0.45)'],
    [0.77,0.46,0.09,0.08,'rgba(75,18,5,0.35)'],
    [0.14,0.43,0.07,0.07,'rgba(100,38,12,0.30)'],
    [0.50,0.35,0.06,0.05,'rgba(85,25,8,0.28)'],
  ];
  for (const [px,py,rx,ry,c] of patches) {
    ctx.save(); ctx.translate(px*W,py*H); ctx.scale(rx*W,ry*H);
    const g = ctx.createRadialGradient(0,0,0,0,0,1);
    g.addColorStop(0,c); g.addColorStop(1,'rgba(0,0,0,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0,0,1,0,Math.PI*2); ctx.fill();
    ctx.restore();
  }
  const ncap = ctx.createRadialGradient(W/2,0,0,W/2,0,H*0.11);
  ncap.addColorStop(0,'rgba(245,245,238,0.97)'); ncap.addColorStop(0.55,'rgba(230,230,220,0.75)'); ncap.addColorStop(1,'rgba(0,0,0,0)');
  ctx.fillStyle = ncap; ctx.fillRect(0,0,W,H*0.15);
  const scap = ctx.createRadialGradient(W/2,H,0,W/2,H,H*0.09);
  scap.addColorStop(0,'rgba(248,248,240,0.95)'); scap.addColorStop(0.55,'rgba(230,228,218,0.65)'); scap.addColorStop(1,'rgba(0,0,0,0)');
  ctx.fillStyle = scap; ctx.fillRect(0,H*0.86,W,H*0.14);
  return new THREE.CanvasTexture(canvas);
}

/** Moon: grey highlands + dark maria + craters */
function makeMoonTex(): THREE.CanvasTexture {
  const W = 1024, H = 512;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#c2c2ba'; ctx.fillRect(0,0,W,H);
  const maria: [number,number,number,number][] = [
    [0.64,0.45,0.13,0.10],[0.57,0.36,0.10,0.08],[0.44,0.40,0.19,0.14],
    [0.24,0.48,0.22,0.16],[0.73,0.55,0.08,0.06],[0.80,0.43,0.07,0.06],[0.60,0.52,0.06,0.05],
  ];
  for (const [mx,my,rx,ry] of maria) {
    ctx.save(); ctx.translate(mx*W,my*H); ctx.scale(rx*W,ry*H);
    const g = ctx.createRadialGradient(0,0,0,0,0,1);
    g.addColorStop(0,'rgba(78,78,72,0.88)'); g.addColorStop(0.55,'rgba(88,86,80,0.68)');
    g.addColorStop(0.88,'rgba(100,98,92,0.32)'); g.addColorStop(1,'rgba(0,0,0,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0,0,1,0,Math.PI*2); ctx.fill();
    ctx.restore();
  }
  const craters: [number,number,number][] = [
    [0.82,0.30,0.025],[0.70,0.28,0.018],[0.50,0.32,0.022],[0.35,0.35,0.015],
    [0.15,0.40,0.020],[0.88,0.58,0.015],[0.40,0.60,0.018],[0.92,0.38,0.012],
  ];
  for (const [cx,cy,cr] of craters) {
    ctx.save(); ctx.translate(cx*W,cy*H);
    ctx.fillStyle = 'rgba(58,58,52,0.38)';
    ctx.beginPath(); ctx.arc(0,0,cr*W*1.1,0,Math.PI*2); ctx.fill();
    const g = ctx.createRadialGradient(0,0,0,0,0,cr*W);
    g.addColorStop(0,'rgba(205,205,195,0.62)'); g.addColorStop(0.65,'rgba(185,183,173,0.28)'); g.addColorStop(1,'rgba(0,0,0,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0,0,cr*W,0,Math.PI*2); ctx.fill();
    ctx.restore();
  }
  const limb = ctx.createRadialGradient(W/2,H/2,W*0.28,W/2,H/2,W*0.54);
  limb.addColorStop(0,'rgba(0,0,0,0)'); limb.addColorStop(0.75,'rgba(0,0,0,0)'); limb.addColorStop(1,'rgba(0,0,0,0.22)');
  ctx.fillStyle = limb; ctx.fillRect(0,0,W,H);
  return new THREE.CanvasTexture(canvas);
}

/** Mercury: dark grey with cratered patches */
function makeMercuryTex(): THREE.CanvasTexture {
  const W = 512, H = 256;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#8a7e70'; ctx.fillRect(0,0,W,H);
  const spots: [number,number,number,string][] = [
    [0.20,0.40,0.12,'rgba(48,38,28,0.32)'],[0.50,0.60,0.10,'rgba(58,48,35,0.28)'],
    [0.75,0.34,0.08,'rgba(118,108,88,0.38)'],[0.90,0.62,0.07,'rgba(48,38,28,0.22)'],
    [0.35,0.28,0.06,'rgba(108,98,78,0.30)'],[0.62,0.45,0.09,'rgba(42,32,22,0.25)'],
  ];
  for (const [px,py,pr,pc] of spots) {
    const g = ctx.createRadialGradient(px*W,py*H,0,px*W,py*H,pr*W);
    g.addColorStop(0,pc); g.addColorStop(1,'rgba(0,0,0,0)');
    ctx.fillStyle = g; ctx.fillRect(0,0,W,H);
  }
  return new THREE.CanvasTexture(canvas);
}

/** Venus: creamy yellow-orange with atmospheric swirls */
function makeVenusTex(): THREE.CanvasTexture {
  const W = 512, H = 256;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const base = ctx.createLinearGradient(0,0,0,H);
  base.addColorStop(0,'#f8e8b0'); base.addColorStop(0.3,'#f0d888');
  base.addColorStop(0.5,'#e8cc78'); base.addColorStop(0.7,'#f0d888');
  base.addColorStop(1,'#f8e8b0');
  ctx.fillStyle = base; ctx.fillRect(0,0,W,H);
  ctx.globalAlpha = 0.20;
  for (let y = 0; y < H; y += 3) {
    const wave = Math.sin(y*0.04)*18 + Math.sin(y*0.016)*12;
    for (let x = 0; x < W; x += 3)
      if (Math.sin((x+wave)*0.025+y*0.012) > 0.5) {
        ctx.fillStyle = Math.sin((x+wave)*0.025+y*0.012) > 0.75 ? 'rgba(215,175,55,1)' : 'rgba(255,240,155,1)';
        ctx.fillRect(x,y,3,3);
      }
  }
  ctx.globalAlpha = 1;
  return new THREE.CanvasTexture(canvas);
}

/** Uranus: pale cyan-green */
function makeUranusTex(): THREE.CanvasTexture {
  const W = 256, H = 128;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const g = ctx.createLinearGradient(0,0,0,H);
  g.addColorStop(0,'#72ccd4'); g.addColorStop(0.35,'#80dce4');
  g.addColorStop(0.5,'#78d2da'); g.addColorStop(0.65,'#80dce4'); g.addColorStop(1,'#72ccd4');
  ctx.fillStyle = g; ctx.fillRect(0,0,W,H);
  return new THREE.CanvasTexture(canvas);
}

/** Neptune: deep blue with cloud bands + Great Dark Spot */
function makeNeptuneTex(): THREE.CanvasTexture {
  const W = 256, H = 128;
  const canvas = document.createElement('canvas');
  canvas.width = W; canvas.height = H;
  const ctx = canvas.getContext('2d')!;
  const g = ctx.createLinearGradient(0,0,0,H);
  g.addColorStop(0,'#2858c8'); g.addColorStop(0.3,'#2048b8');
  g.addColorStop(0.5,'#3060d0'); g.addColorStop(0.7,'#2048b8'); g.addColorStop(1,'#2858c8');
  ctx.fillStyle = g; ctx.fillRect(0,0,W,H);
  ctx.globalAlpha = 0.32;
  for (let y = 0; y < H; y += 4) {
    const wave = Math.sin(y*0.08)*6;
    for (let x = 0; x < W; x += 4)
      if (Math.sin((x+wave)*0.05+y*0.02) > 0.62) {
        ctx.fillStyle = 'rgba(148,188,238,1)'; ctx.fillRect(x,y,4,2);
      }
  }
  ctx.globalAlpha = 1;
  ctx.save(); ctx.translate(W*0.55,H*0.45); ctx.scale(1,0.5);
  const ds = ctx.createRadialGradient(0,0,0,0,0,W*0.09);
  ds.addColorStop(0,'rgba(18,28,95,0.88)'); ds.addColorStop(0.7,'rgba(18,28,95,0.4)'); ds.addColorStop(1,'rgba(0,0,0,0)');
  ctx.fillStyle = ds; ctx.beginPath(); ctx.arc(0,0,W*0.09,0,Math.PI*2); ctx.fill();
  ctx.restore();
  return new THREE.CanvasTexture(canvas);
}

// ─── Globe ───────────────────────────────────────────────────────────────────

function GlobeMesh() {
  const { gl } = useThree();
  const [earthTex, bumpTex, specTex] = useMemo(() => {
    const loader = new THREE.TextureLoader();
    const maxAniso = gl.capabilities.getMaxAnisotropy();
    const load = (url: string) => {
      const t = loader.load(url);
      t.anisotropy = maxAniso;
      return t;
    };
    return [load(EARTH_MAP), load(EARTH_BUMP), load(EARTH_SPEC)];
  }, [gl]);

  return (
    <mesh>
      <sphereGeometry args={[2, 128, 128]} />
      <meshPhongMaterial
        map={earthTex}
        bumpMap={bumpTex}
        bumpScale={0.18}
        specularMap={specTex}
        specular={new THREE.Color(0x226688)}
        shininess={12}
        emissive={new THREE.Color(0x1f3355)}
        emissiveIntensity={0.22}
      />
    </mesh>
  );
}

function Globe() {
  return (
    <>
      <mesh scale={1.015}>
        <sphereGeometry args={[2, 32, 32]} />
        <meshStandardMaterial color="#88aaff" transparent opacity={0.12} side={THREE.BackSide} />
      </mesh>
      <mesh scale={1.045}>
        <sphereGeometry args={[2, 32, 32]} />
        <meshStandardMaterial color="#2255bb" transparent opacity={0.06} side={THREE.BackSide} />
      </mesh>
    </>
  );
}

// ─── Sun & Moon ──────────────────────────────────────────────────────────────

function Sun() {
  const groupRef = useRef<THREE.Group>(null);
  useFrame(({ clock }) => {
    if (groupRef.current) {
      const t = clock.elapsedTime * 0.04;
      groupRef.current.position.set(Math.cos(t) * 18, 4, Math.sin(t) * 18);
    }
  });

  const coronaMat = useMemo(() => new THREE.MeshBasicMaterial({
    color: '#fff4a0',
    transparent: true,
    opacity: 0.15,
    side: THREE.BackSide,
  }), []);

  return (
    <group ref={groupRef} position={[18, 4, 0]}>
      <mesh>
        <sphereGeometry args={[0.45, 24, 24]} />
        <meshBasicMaterial color="#ffe060" />
      </mesh>
      <mesh scale={1.6}>
        <sphereGeometry args={[0.45, 16, 16]} />
        <primitive object={coronaMat} />
      </mesh>
      <pointLight intensity={4.5} color="#fff8e0" distance={80} decay={1.2} />
    </group>
  );
}

function Moon() {
  const groupRef = useRef<THREE.Group>(null);
  const moonMap = useMemo(() => makeMoonTex(), []);
  useFrame(({ clock }) => {
    if (groupRef.current) {
      const t = clock.elapsedTime * 0.28;
      groupRef.current.position.set(
        Math.cos(t) * 4.2,
        Math.sin(t * 0.4) * 0.6,
        Math.sin(t) * 4.2,
      );
    }
  });

  return (
    <group ref={groupRef}>
      <mesh>
        <sphereGeometry args={[0.2, 32, 32]} />
        <meshStandardMaterial map={moonMap} color="#ffffff" roughness={0.95} metalness={0.01} />
      </mesh>
      <pointLight intensity={0.08} color="#c8d8ff" distance={10} />
    </group>
  );
}

// ─── Solar System ─────────────────────────────────────────────────────────────

/** Faint orbit ring drawn in the ecliptic plane. */
function OrbitPath({ dist }: { dist: number }) {
  const line = useMemo(() => {
    const pts: number[] = [];
    const N = 256;
    for (let i = 0; i <= N; i++) {
      const a = (i / N) * Math.PI * 2;
      pts.push(Math.cos(a) * dist, 0, Math.sin(a) * dist);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
    const mat = new THREE.LineBasicMaterial({ color: '#1a2a5a', transparent: true, opacity: 0.20 });
    return new THREE.Line(geo, mat);
  }, [dist]);

  return <primitive object={line} />;
}

interface PlanetProps {
  radius: number;
  dist: number;
  speed: number;
  incl: number;
  color: string;
  roughness?: number;
  map?: THREE.CanvasTexture;
  ringTex?: THREE.CanvasTexture;
}

function Planet({ radius, dist, speed, incl, color, roughness = 0.75, map, ringTex }: PlanetProps) {
  const ref = useRef<THREE.Group>(null);
  useFrame(({ clock }) => {
    if (!ref.current) return;
    const t = clock.elapsedTime * speed;
    ref.current.position.set(
      Math.cos(t) * dist,
      Math.sin(t * 0.35) * dist * incl,
      Math.sin(t) * dist,
    );
  });

  return (
    <group ref={ref}>
      <mesh>
        <sphereGeometry args={[radius, 36, 36]} />
        <meshStandardMaterial
          map={map}
          color={map ? '#ffffff' : color}
          roughness={roughness}
          metalness={0.04}
        />
      </mesh>
      {ringTex && (
        <mesh rotation={[Math.PI * 0.46, 0.18, 0.14]}>
          <ringGeometry args={[radius * 1.38, radius * 2.55, 128]} />
          <meshBasicMaterial map={ringTex} transparent side={THREE.DoubleSide} />
        </mesh>
      )}
    </group>
  );
}

function SolarSystem() {
  const mercuryMap = useMemo(() => makeMercuryTex(), []);
  const venusMap   = useMemo(() => makeVenusTex(),   []);
  const marsMap    = useMemo(() => makeMarsTex(),     []);
  const jupiterMap = useMemo(() => makeJupiterTex(),  []);
  const saturnMap  = useMemo(() => makeSaturnTex(),   []);
  const saturnRing = useMemo(() => makeSaturnRingTex(),[]);
  const uranusMap  = useMemo(() => makeUranusTex(),   []);
  const neptuneMap = useMemo(() => makeNeptuneTex(),  []);

  return (
    <>
      {/* Mercury */}
      <group key="mercury">
        <OrbitPath dist={22} />
        <Planet radius={0.10} dist={22} speed={0.074} incl={0.08} color="#8a7e70" roughness={0.92} map={mercuryMap} />
      </group>
      {/* Venus */}
      <group key="venus">
        <OrbitPath dist={30} />
        <Planet radius={0.20} dist={30} speed={0.050} incl={0.04} color="#f0d888" roughness={0.55} map={venusMap} />
      </group>
      {/* Mars */}
      <group key="mars">
        <OrbitPath dist={45} />
        <Planet radius={0.13} dist={45} speed={0.028} incl={0.17} color="#c04828" roughness={0.88} map={marsMap} />
      </group>
      {/* Jupiter */}
      <group key="jupiter">
        <OrbitPath dist={75} />
        <Planet radius={0.75} dist={75} speed={0.012} incl={0.06} color="#c2956a" roughness={0.55} map={jupiterMap} />
      </group>
      {/* Saturn + gradient rings */}
      <group key="saturn">
        <OrbitPath dist={110} />
        <Planet radius={0.60} dist={110} speed={0.008} incl={0.18} color="#d4b870" roughness={0.50} map={saturnMap} ringTex={saturnRing} />
      </group>
      {/* Uranus */}
      <group key="uranus">
        <OrbitPath dist={148} />
        <Planet radius={0.38} dist={148} speed={0.005} incl={0.48} color="#78d4dc" roughness={0.40} map={uranusMap} />
      </group>
      {/* Neptune */}
      <group key="neptune">
        <OrbitPath dist={185} />
        <Planet radius={0.36} dist={185} speed={0.003} incl={0.10} color="#2858c8" roughness={0.40} map={neptuneMap} />
      </group>
    </>
  );
}

// ─── Globe grid & population ──────────────────────────────────────────────────

function GridLines() {
  const geo = useMemo(() => {
    const pts: number[] = [];
    const r = 2.005;
    for (let lat = -80; lat <= 80; lat += 20) {
      const phi = (lat * Math.PI) / 180;
      for (let i = 0; i <= 128; i++) {
        const lon = (i / 128) * 2 * Math.PI;
        pts.push(r * Math.cos(phi) * Math.cos(lon), r * Math.sin(phi), -r * Math.cos(phi) * Math.sin(lon));
      }
    }
    for (let lon = 0; lon < 360; lon += 30) {
      const theta = (lon * Math.PI) / 180;
      for (let i = 0; i <= 64; i++) {
        const phi = ((i / 64) * 180 - 90) * (Math.PI / 180);
        pts.push(r * Math.cos(phi) * Math.cos(theta), r * Math.sin(phi), -r * Math.cos(phi) * Math.sin(theta));
      }
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
    return g;
  }, []);

  return (
    <lineSegments geometry={geo}>
      <lineBasicMaterial color="#1a2a5a" transparent opacity={0.25} />
    </lineSegments>
  );
}

function buildPositions(individuals: { x?: number; y?: number }[], r: number): THREE.BufferGeometry {
  const positions: number[] = [];
  for (const ind of individuals) {
    const lat = ((ind.y ?? 0) * Math.PI) / 180;
    const lon = ((ind.x ?? 0) * Math.PI) / 180;
    positions.push(
      r * Math.cos(lat) * Math.cos(lon),
      r * Math.sin(lat),
      -r * Math.cos(lat) * Math.sin(lon),
    );
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(positions.length ? positions : [0, 0, 0], 3));
  return g;
}

function PopulationDots({
  individuals,
}: {
  individuals: any[];
  groupRef: React.RefObject<THREE.Group | null>;
  onSelect?: (ind: any) => void;
}) {
  const spriteTex = useMemo(() => getSharedSpriteTexture(), []);
  const alive = useMemo(() => individuals.filter(i => !i.is_dead && i.alive !== false), [individuals]);
  const founderGeo = useMemo(() => buildPositions(alive.filter(i => i.is_founder), 2.025), [alive]);
  const maleGeo    = useMemo(() => buildPositions(alive.filter(i => !i.is_founder && i.sex === 'male'), 2.025), [alive]);
  const femaleGeo  = useMemo(() => buildPositions(alive.filter(i => !i.is_founder && i.sex !== 'male'), 2.025), [alive]);
  // Always keep all three <points> mounted — conditional unmounting causes R3F
  // to dispose materials and can corrupt the shared sprite texture for siblings.
  // Empty groups fall back to a single invisible point at [0,0,0] (inside globe).
  // renderOrder={2} forces population dots to draw after centroid trail (renderOrder 1)
  // and pop trails (renderOrder 1), regardless of Three.js camera-distance sort.
  return (
    <>
      <points renderOrder={2} geometry={founderGeo}>
        <pointsMaterial map={spriteTex} size={0.12} color="#ff4444" sizeAttenuation transparent opacity={0.95} depthWrite={false} alphaTest={0.1} />
      </points>
      <points renderOrder={2} geometry={maleGeo}>
        <pointsMaterial map={spriteTex} size={0.09} color="#90caff" sizeAttenuation transparent opacity={0.92} depthWrite={false} alphaTest={0.1} />
      </points>
      <points renderOrder={2} geometry={femaleGeo}>
        <pointsMaterial map={spriteTex} size={0.09} color="#ffaacc" sizeAttenuation transparent opacity={0.92} depthWrite={false} alphaTest={0.1} />
      </points>
    </>
  );
}

function PopulationTrails({ snapshots }: { snapshots: { x: number; y: number }[][] }) {
  const spriteTex = useMemo(() => getSharedSpriteTexture(), []);
  const layers = useMemo(() => snapshots.map((snapshot, idx) => {
    const age = snapshots.length - 1 - idx;
    return {
      geo: buildPositions(snapshot, 2.023),
      opacity: Math.max(0.04, 0.35 - age * 0.06),
      size: Math.max(0.035, 0.08 - age * 0.008),
    };
  }), [snapshots]);

  return (
    <>
      {layers.map((layer, i) => (
        <points key={i} renderOrder={1} geometry={layer.geo}>
          <pointsMaterial map={spriteTex} size={layer.size} color="#88aaee" sizeAttenuation transparent opacity={layer.opacity} depthWrite={false} alphaTest={0.1} />
        </points>
      ))}
    </>
  );
}

function GlobeClickCatcher({
  groupRef,
  onGlobeClick,
}: {
  groupRef: React.RefObject<THREE.Group | null>;
  onGlobeClick: (lat: number, lon: number) => void;
}) {
  return (
    <mesh
      onClick={(e) => {
        e.stopPropagation();
        if (!groupRef.current) return;
        const local = groupRef.current.worldToLocal(e.point.clone());
        const r = local.length();
        const lat = (Math.asin(local.y / r) * 180) / Math.PI;
        const lon = (Math.atan2(-local.z, local.x) * 180) / Math.PI;
        onGlobeClick(Math.round(lat * 1000) / 1000, Math.round(lon * 1000) / 1000);
      }}
    >
      <sphereGeometry args={[2.01, 32, 32]} />
      <meshBasicMaterial transparent opacity={0.001} depthWrite={false} />
    </mesh>
  );
}

function PopClickCatcher({
  individuals,
  groupRef,
  onSelect,
}: {
  individuals: any[];
  groupRef: React.RefObject<THREE.Group | null>;
  onSelect?: (ind: any) => void;
}) {
  if (!onSelect || individuals.length === 0) return null;
  return (
    <mesh
      onClick={(e) => {
        if (!onSelect || individuals.length === 0) return;
        const r = 2.025;
        let minDist = Infinity, minIdx = -1;
        const ry = groupRef.current?.rotation.y ?? 0;
        individuals.forEach((ind, i) => {
          const lat = ((ind.y ?? 0) * Math.PI) / 180;
          const lon = ((ind.x ?? 0) * Math.PI) / 180;
          const px = r * Math.cos(lat) * Math.cos(lon);
          const py = r * Math.sin(lat);
          const pz = -r * Math.cos(lat) * Math.sin(lon);
          const wx = px * Math.cos(ry) + pz * Math.sin(ry);
          const wz = -px * Math.sin(ry) + pz * Math.cos(ry);
          const d = Math.hypot(wx - e.point.x, py - e.point.y, wz - e.point.z);
          if (d < minDist) { minDist = d; minIdx = i; }
        });
        if (minIdx >= 0 && minDist < 0.06) { e.stopPropagation(); onSelect(individuals[minIdx]); }
      }}
    >
      <sphereGeometry args={[2.03, 32, 32]} />
      <meshBasicMaterial transparent opacity={0.001} depthWrite={false} />
    </mesh>
  );
}

const TRAIL_DEPTH = 6;

function latLonToVec3(x: number, y: number, r: number): [number, number, number] {
  const lat = (y * Math.PI) / 180;
  const lon = (x * Math.PI) / 180;
  return [r * Math.cos(lat) * Math.cos(lon), r * Math.sin(lat), -r * Math.cos(lat) * Math.sin(lon)];
}

function CentroidTrailLine({ trail }: { trail: CentroidPoint[] }) {
  // Use the shared sprite texture so centroid dots are circular, not squares.
  // renderOrder={1} ensures they always draw before population dots (renderOrder 2)
  // regardless of Three.js's camera-distance transparency sort.
  const spriteTex = useMemo(() => getSharedSpriteTexture(), []);

  const line = useMemo(() => {
    if (trail.length < 2) return null;
    const r = 2.032;
    const pts: number[] = [];
    for (const pt of trail) {
      const [x, y, z] = latLonToVec3(pt.x, pt.y, r);
      pts.push(x, y, z);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
    const mat = new THREE.LineBasicMaterial({ color: '#fbbf24', transparent: true, opacity: 0.7, linewidth: 2 });
    return new THREE.Line(geo, mat);
  }, [trail]);

  const dots = useMemo(() => {
    if (trail.length === 0) return null;
    const r = 2.034;
    const pts: number[] = [];
    for (const pt of trail) {
      const [x, y, z] = latLonToVec3(pt.x, pt.y, r);
      pts.push(x, y, z);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
    return geo;
  }, [trail]);

  if (!line && !dots) return null;
  return (
    <>
      {line && <primitive object={line} />}
      {dots && (
        <points renderOrder={1} geometry={dots}>
          <pointsMaterial map={spriteTex} size={0.06} color="#fbbf24" sizeAttenuation transparent opacity={0.9} depthWrite={false} alphaTest={0.1} />
        </points>
      )}
    </>
  );
}

function RotatingGroup({
  individuals,
  onSelect,
  onGlobeClick,
  globeRotRef,
}: {
  individuals: any[];
  onSelect?: (ind: any) => void;
  onGlobeClick?: (lat: number, lon: number) => void;
  globeRotRef?: React.RefObject<number>;
}) {
  const groupRef = useRef<THREE.Group>(null);
  const centroidTrail = useSimStore(s => s.centroidTrail);
  const autoRotate = useSimStore(s => s.globeAutoRotate);

  useFrame((_, delta) => {
    if (!groupRef.current) return;
    if (autoRotate) {
      groupRef.current.rotation.y += delta * 0.025;
    }
    if (globeRotRef) (globeRotRef as React.MutableRefObject<number>).current = groupRef.current.rotation.y;
  });

  const prevIndRef = useRef<any[]>([]);
  const [trailSnapshots, setTrailSnapshots] = useState<{ x: number; y: number }[][]>([]);

  useEffect(() => {
    const prev = prevIndRef.current;
    if (prev.length > 0) {
      const snapshot = prev.map(ind => ({ x: ind.x ?? 0, y: ind.y ?? 0 }));
      setTrailSnapshots(s => [...s, snapshot].slice(-TRAIL_DEPTH));
    }
    prevIndRef.current = individuals;
  }, [individuals]);

  return (
    <group ref={groupRef}>
      <GlobeMesh />
      <GridLines />
      {centroidTrail.length > 1 && <CentroidTrailLine trail={centroidTrail} />}
      {onGlobeClick && <GlobeClickCatcher groupRef={groupRef} onGlobeClick={onGlobeClick} />}
      {individuals.length > 0 && (
        <>
          {trailSnapshots.length > 0 && <PopulationTrails snapshots={trailSnapshots} />}
          <PopulationDots individuals={individuals} groupRef={groupRef} onSelect={onSelect} />
          <PopClickCatcher individuals={individuals} groupRef={groupRef} onSelect={onSelect} />
        </>
      )}
    </group>
  );
}

function IntroCameraRig({
  introPlaying,
  spawnLat,
  spawnLon,
  globeRotRef,
  onIntroComplete,
  controlsRef,
}: {
  introPlaying: boolean;
  spawnLat: number;
  spawnLon: number;
  globeRotRef: React.RefObject<number>;
  onIntroComplete?: () => void;
  controlsRef: React.RefObject<any>;
}) {
  const { camera } = useThree();
  const progressRef = useRef(0);
  const completeRef = useRef(false);

  useEffect(() => {
    progressRef.current = 0;
    completeRef.current = false;
  }, [introPlaying]);

  useFrame((_, delta) => {
    if (!introPlaying) return;
    progressRef.current = Math.min(1, progressRef.current + delta / 2.8);
    const eased = 1 - Math.pow(1 - progressRef.current, 3);
    const latR = (spawnLat * Math.PI) / 180;
    const lonR = (spawnLon * Math.PI) / 180 + ((globeRotRef as React.MutableRefObject<number>).current ?? 0);
    const CAM_DIST = 5.5;
    const endPos = new THREE.Vector3(
      CAM_DIST * Math.cos(latR) * Math.cos(lonR),
      CAM_DIST * Math.sin(latR),
      -CAM_DIST * Math.cos(latR) * Math.sin(lonR),
    );
    camera.position.lerpVectors(new THREE.Vector3(0, 1.15, 38), endPos, eased);
    camera.lookAt(0, 0, 0);
    controlsRef.current?.target?.set?.(0, 0, 0);
    controlsRef.current?.update?.();
    if (progressRef.current >= 1 && !completeRef.current) {
      completeRef.current = true;
      onIntroComplete?.();
    }
  });

  return null;
}

// ─── Main export ──────────────────────────────────────────────────────────────

function WorldGlobe({
  individuals = [],
  spawnLat = 39,
  spawnLon = 28,
  onSelect,
  onGlobeClick,
  introPlaying = false,
  onIntroComplete,
}: {
  individuals?: any[];
  spawnLat?: number;
  spawnLon?: number;
  onSelect?: (ind: any) => void;
  onGlobeClick?: (lat: number, lon: number) => void;
  introPlaying?: boolean;
  onIntroComplete?: () => void;
}) {
  const controlsRef = useRef<any>(null);
  const globeRotRef = useRef<number>(0);

  useEffect(() => {
    if (controlsRef.current) {
      controlsRef.current.enabled = !introPlaying;
      controlsRef.current.target.set(0, 0, 0);
      controlsRef.current.update?.();
    }
  }, [introPlaying]);

  return (
    <Canvas
      camera={{ position: introPlaying ? [0, 1.15, 38] : [0, 0.5, 5.5], fov: introPlaying ? 34 : 42 }}
      style={{ background: 'transparent' }}
      dpr={Math.min(window.devicePixelRatio, 3)}
      gl={{ antialias: true, alpha: true, logarithmicDepthBuffer: true, powerPreference: 'high-performance' }}
    >
      <ambientLight intensity={2.8} />
      <directionalLight position={[5, 3, 6]} intensity={1.8} color="#fff8f0" />
      <directionalLight position={[-5, 3, -4]} intensity={1.3} color="#d0e8ff" />
      <directionalLight position={[0, -5, 4]} intensity={1.0} color="#ffffff" />
      <Sun />
      <Moon />
      <SolarSystem />
      <Stars radius={250} depth={80} count={6000} factor={5} saturation={0} fade speed={0.3} />
      <Globe />
      <IntroCameraRig
        introPlaying={introPlaying}
        spawnLat={spawnLat}
        spawnLon={spawnLon}
        globeRotRef={globeRotRef}
        onIntroComplete={onIntroComplete}
        controlsRef={controlsRef}
      />
      <RotatingGroup
        individuals={individuals}
        onSelect={onSelect}
        onGlobeClick={onGlobeClick}
        globeRotRef={globeRotRef}
      />
      <OrbitControls
        ref={controlsRef}
        enablePan={false}
        minDistance={2.15}
        maxDistance={220}
        rotateSpeed={0.45}
        zoomSpeed={0.7}
        dampingFactor={0.08}
        enableDamping
      />
    </Canvas>
  );
}

// Only re-render when data props change — ignore inline callback identity changes.
// This prevents notification-triggered SimulationPage re-renders from propagating
// into the Three.js scene and corrupting material state.
export default memo(WorldGlobe, (prev, next) =>
  prev.individuals === next.individuals &&
  prev.spawnLat === next.spawnLat &&
  prev.spawnLon === next.spawnLon &&
  prev.introPlaying === next.introPlaying
);
