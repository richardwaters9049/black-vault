import { useEffect, useRef } from 'react';
import countryBorders from '../data/countryBorders.json';

// Natural Earth 1:110m administrative country boundaries, simplified and bundled
// locally. Source: Natural Earth (public domain), ne_110m_admin_0_countries.
type Coordinate = [number, number];
const rad = Math.PI / 180;
const borders: Coordinate[][] = (countryBorders as number[][][]).map(ring =>
  ring.map(([longitude, latitude]) => [longitude * rad, latitude * rad] as Coordinate),
);
function inside(x: number, y: number, polygon: Coordinate[]) {
  let hit = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const [xi, yi] = polygon[i]; const [xj, yj] = polygon[j];
    if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}
// Country-specific land points retain genuine coastlines rather than approximate
// hand-drawn continent silhouettes. Generate once, never during animation.
const dots: Coordinate[] = [];
for (const ring of borders) {
  const lon = ring.map(point => point[0]);
  const lat = ring.map(point => point[1]);
  const minX = Math.min(...lon), maxX = Math.max(...lon);
  const minY = Math.min(...lat), maxY = Math.max(...lat);
  for (let y = Math.ceil(minY / (.027)) * .027; y <= maxY; y += .027) {
    for (let x = Math.ceil(minX / (.027)) * .027; x <= maxX; x += .027) {
      if (inside(x, y, ring)) dots.push([x, y]);
    }
  }
}
const landmarks: [number, number][] = [
  [-74,40.7],[-0.12,51.5],[2.35,48.86],[13.4,52.5],[28.98,41.0],[37.6,55.7],
  [31.2,30.0],[3.4,6.45],[18.4,-33.9],[55.3,25.2],[72.9,19.1],[77.2,28.6],
  [103.8,1.35],[116.4,39.9],[121.5,31.2],[139.7,35.7],[151.2,-33.9],
  [-46.6,-23.5],[-58.4,-34.6],[-99.1,19.4],[-118.2,34.1]
].map(([lon,lat]) => [lon * rad, lat * rad]);
function project(lon: number, lat: number, turn: number, tilt: number, radius: number) {
  const a = lon - turn;
  const depth = Math.cos(lat) * Math.cos(a);
  const x = Math.cos(lat) * Math.sin(a);
  const y = Math.sin(lat) * Math.cos(tilt) - depth * Math.sin(tilt);
  const z = Math.sin(lat) * Math.sin(tilt) + depth * Math.cos(tilt);
  return { x: x * radius, y: -y * radius, z };
}
export default function AuthGlobe({ cinematic = false }: { cinematic?: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    const ctx = element?.getContext('2d');
    if (!element || !ctx) return;
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
    let frame = 0; let previous = 0; let angle = -0.3; let entranceStart = 0;
    const render = (now: number) => {
      const delta = previous ? Math.min(now - previous, 100) : 0;
      if (!cinematic && delta < 32 && previous !== 0) { frame = requestAnimationFrame(render); return; }
      previous = now;
      if (!entranceStart) entranceStart = now + (cinematic ? 2050 : 0);
      const elapsed = Math.max(now - entranceStart, 0);

      let turn: number;
      if (cinematic) {
        // One uninterrupted accelerating rotation for the full camera push.
        const seconds = elapsed / 1000;
        const baseVelocity = 1.15;
        const acceleration = 0.72;
        turn = reduce.matches
          ? angle
          : angle + baseVelocity * seconds + 0.5 * acceleration * seconds * seconds;
      } else {
        if (!reduce.matches) angle += delta * 0.00012;
        const entrance = reduce.matches ? 1 : Math.min(elapsed / 1450, 1);
        const spin = reduce.matches ? 0 : -Math.PI * 1.3 * Math.pow(1 - entrance, 3);
        turn = angle + spin;
      }
      const width = element.clientWidth;
      const height = element.clientHeight;
      if (width === 0 || height === 0) return;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const pixelWidth = Math.round(width * dpr), pixelHeight = Math.round(height * dpr);
      if (element.width !== pixelWidth || element.height !== pixelHeight) {
        element.width = pixelWidth; element.height = pixelHeight;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, width, height);
      const cx = width / 2, cy = height / 2, r = Math.min(width, height) * .405, tilt = 0.18;
      const sphere = ctx.createRadialGradient(cx - r * .3, cy - r * .3, r * .04, cx, cy, r);
      sphere.addColorStop(0, '#112b4a'); sphere.addColorStop(.65, '#0b1a30'); sphere.addColorStop(1, '#050b15');
      ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fillStyle = sphere; ctx.fill();
      ctx.strokeStyle = 'rgba(96,165,250,.22)'; ctx.lineWidth = .8; ctx.stroke();
      const strokeCurve = (points: [number, number][], alpha: number) => {
        ctx.beginPath(); let active = false;
        for (const [lon, lat] of points) {
          const p = project(lon, lat, turn, tilt, r);
          if (p.z <= .01) { active = false; continue; }
          if (!active) ctx.moveTo(cx + p.x, cy + p.y); else ctx.lineTo(cx + p.x, cy + p.y);
          active = true;
        }
        ctx.strokeStyle = 'rgba(96,165,250,' + alpha + ')'; ctx.lineWidth = .65; ctx.stroke();
      };
      for (let lat = -75; lat <= 75; lat += 10) {
        strokeCurve(Array.from({ length: 145 }, (_, i) => [-Math.PI + i * Math.PI / 72, lat * rad]), .13);
      }
      for (let lon = -180; lon < 180; lon += 10) {
        strokeCurve(Array.from({ length: 73 }, (_, i) => [lon * rad, (-90 + i * 2.5) * rad]), .11);
      }
      // Actual national boundaries, including coastlines and island outlines.
      // Clip each segment to the visible hemisphere rather than drawing lines
      // through the back of the globe.
      ctx.lineWidth = .68;
      for (const ring of borders) {
        ctx.beginPath();
        let previousPoint: ReturnType<typeof project> | null = null;
        for (const [lon, lat] of ring) {
          const current = project(lon, lat, turn, tilt, r);
          if (previousPoint) {
            const frontA = previousPoint.z > 0;
            const frontB = current.z > 0;
            if (frontA || frontB) {
              if (frontA !== frontB) {
                const fraction = previousPoint.z / (previousPoint.z - current.z);
                const ix = previousPoint.x + (current.x - previousPoint.x) * fraction;
                const iy = previousPoint.y + (current.y - previousPoint.y) * fraction;
                if (frontA) {
                  ctx.moveTo(cx + previousPoint.x, cy + previousPoint.y);
                  ctx.lineTo(cx + ix, cy + iy);
                } else {
                  ctx.moveTo(cx + ix, cy + iy);
                  ctx.lineTo(cx + current.x, cy + current.y);
                }
              } else {
                ctx.moveTo(cx + previousPoint.x, cy + previousPoint.y);
                ctx.lineTo(cx + current.x, cy + current.y);
              }
            }
          }
          previousPoint = current;
        }
        ctx.strokeStyle = 'rgba(126,184,255,.48)';
        ctx.stroke();
      }
      // Delicate continent silhouettes: connect sampled land dots without a raster texture.
      for (const [lon, lat] of dots) {
        const p = project(lon, lat, turn, tilt, r);
        if (p.z < .03) continue;
        const opacity = .34 + p.z * .62;
        ctx.fillStyle = 'rgba(119,174,255,' + opacity + ')';
        ctx.beginPath(); ctx.arc(cx + p.x, cy + p.y, .46 + p.z * .3, 0, Math.PI * 2); ctx.fill();
      }
      // A restrained dotted equatorial band and two polar guide circles.
      for (const latitude of [-66.5, 0, 66.5]) {
        for (let longitude = -180; longitude < 180; longitude += 3) {
          const p = project(longitude * rad, latitude * rad, turn, tilt, r);
          if (p.z <= .02) continue;
          ctx.fillStyle = latitude === 0 ? 'rgba(147,197,253,.22)' : 'rgba(147,197,253,.13)';
          ctx.beginPath(); ctx.arc(cx + p.x, cy + p.y, .7, 0, Math.PI * 2); ctx.fill();
        }
      }
      // Sparse luminous network nodes and their short local connections.
      for (let i = 0; i < landmarks.length; i++) {
        const [lon, lat] = landmarks[i];
        const a = project(lon, lat, turn, tilt, r);
        if (a.z <= .06) continue;
        const halo = ctx.createRadialGradient(cx + a.x, cy + a.y, 0, cx + a.x, cy + a.y, 9);
        halo.addColorStop(0, 'rgba(147,197,253,.40)');
        halo.addColorStop(1, 'rgba(59,130,246,0)');
        ctx.fillStyle = halo; ctx.beginPath(); ctx.arc(cx + a.x, cy + a.y, 9, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = 'rgba(191,219,254,.92)';
        ctx.beginPath(); ctx.arc(cx + a.x, cy + a.y, 1.45, 0, Math.PI * 2); ctx.fill();
        const next = landmarks[(i + 1) % landmarks.length];
        const b = project(next[0], next[1], turn, tilt, r);
        if (b.z > .08 && Math.hypot(a.x - b.x, a.y - b.y) < r * .42) {
          ctx.beginPath(); ctx.moveTo(cx + a.x, cy + a.y); ctx.lineTo(cx + b.x, cy + b.y);
          ctx.strokeStyle = 'rgba(125,180,255,.16)'; ctx.lineWidth = .65; ctx.stroke();
        }
      }
      // Thin terminator and luminous crescent at the right-hand edge.
      ctx.save(); ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.clip();
      const terminator = ctx.createLinearGradient(cx - r, cy, cx + r, cy);
      terminator.addColorStop(0, 'rgba(0,0,0,0)');
      terminator.addColorStop(.58, 'rgba(0,0,0,.03)');
      terminator.addColorStop(1, 'rgba(0,0,0,.50)');
      ctx.fillStyle = terminator; ctx.fillRect(cx-r, cy-r, 2*r, 2*r);
      ctx.restore();
      ctx.beginPath(); ctx.arc(cx, cy, r + .6, -.98, 1.12);
      ctx.strokeStyle = 'rgba(125,179,255,.46)'; ctx.lineWidth = 1.25; ctx.stroke();
      // A quiet atmospheric rim gives depth without competing with the form.
      const glow = ctx.createRadialGradient(cx, cy, r * .89, cx, cy, r * 1.12);
      glow.addColorStop(0, 'rgba(59,130,246,0)'); glow.addColorStop(.8, 'rgba(59,130,246,.12)'); glow.addColorStop(1, 'rgba(59,130,246,0)');
      ctx.fillStyle = glow; ctx.beginPath(); ctx.arc(cx, cy, r * 1.12, 0, Math.PI * 2); ctx.fill();
      if (!reduce.matches) frame = requestAnimationFrame(render);
    };
    frame = requestAnimationFrame(render);
    const changed = () => { cancelAnimationFrame(frame); previous = 0; frame = requestAnimationFrame(render); };
    reduce.addEventListener('change', changed);
    return () => { cancelAnimationFrame(frame); reduce.removeEventListener('change', changed); };
  }, [cinematic]);
  return <canvas ref={canvas} aria-hidden="true" className="absolute inset-0 size-full" />;
}
