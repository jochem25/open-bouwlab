/**
 * 3D van de gegenereerde trap: het IFC (met clearance) geladen met web-ifc via
 * lib/bengShellLoader (kleur en doorzicht uit het IFC), in dezelfde
 * OBC-wereld als IfcImportViewer3D. Gekozen langs het pad van de viewer-spike
 * (O-2026-10-30-02): web-ifc 0.0.77, geen Fragments.
 *
 * De clearance zit twee keer in het bestand (representatie op de IfcStair en
 * de proxy 'Clearance'); de schakelaar verbergt of toont beide.
 */
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import * as THREE from "three";
import * as OBC from "@thatopen/components";

import { loadBengShell, type BengShellElement } from "../../lib/bengShellLoader";

const WASM_PATH = "/wasm/";
/** Hoek waarboven een rand als lijn wordt getekend (graden). */
const RAND_HOEK = 30;
const ACHTERGROND = 0xf5f5f4;
const RANDKLEUR = 0x1f2937;
/** Kijkrichting van het isometrische overzicht. */
const ISO = new THREE.Vector3(1, 0.8, 1).normalize();

function isClearance(el: BengShellElement): boolean {
  return el.ifcType === "IfcStair" || (el.ifcType === "IfcBuildingElementProxy" && el.name === "Clearance");
}

interface Props {
  ifc: string;
  toonClearance: boolean;
}

export function TrapViewer3D({ ifc, toonClearance }: Props) {
  const { t } = useTranslation();
  const container = useRef<HTMLDivElement | null>(null);
  const clearanceGroep = useRef<THREE.Group | null>(null);
  const [status, setStatus] = useState<"laden" | "klaar" | "fout">("laden");
  const [fout, setFout] = useState<string | null>(null);
  // de schakelaar mag het model niet opnieuw laden
  const toonClearanceRef = useRef(toonClearance);
  toonClearanceRef.current = toonClearance;

  useEffect(() => {
    const el = container.current;
    if (!el) return;
    let actief = true;
    const components = new OBC.Components();
    const world = components.get(OBC.Worlds).create<OBC.SimpleScene, OBC.SimpleCamera, OBC.SimpleRenderer>();
    world.scene = new OBC.SimpleScene(components);
    world.renderer = new OBC.SimpleRenderer(components, el);
    world.camera = new OBC.SimpleCamera(components);
    components.init();
    world.scene.setup();
    const scene = world.scene.three;
    scene.background = new THREE.Color(ACHTERGROND);
    setStatus("laden");
    setFout(null);

    const bytes = new TextEncoder().encode(ifc);
    loadBengShell(bytes, { wasmPath: WASM_PATH, coordinateToOrigin: false })
      .then(async (els) => {
        if (!actief) return;
        const trap = new THREE.Group();
        const clearance = new THREE.Group();
        for (const e of els) {
          const doel = isClearance(e) ? clearance : trap;
          for (const p of e.parts) {
            const geo = new THREE.BufferGeometry();
            geo.setAttribute("position", new THREE.BufferAttribute(p.positions, 3));
            geo.setIndex(new THREE.BufferAttribute(p.indices, 1));
            geo.computeVertexNormals();
            const mat = new THREE.MeshLambertMaterial({
              color: p.color,
              transparent: p.opacity < 1,
              opacity: p.opacity,
              side: THREE.DoubleSide,
              depthWrite: p.opacity >= 1,
            });
            doel.add(new THREE.Mesh(geo, mat));
            if (doel === trap) {
              doel.add(
                new THREE.LineSegments(
                  new THREE.EdgesGeometry(geo, RAND_HOEK),
                  new THREE.LineBasicMaterial({ color: RANDKLEUR, transparent: true, opacity: 0.5 }),
                ),
              );
            }
          }
        }
        scene.add(trap);
        scene.add(clearance);
        clearanceGroep.current = clearance;
        clearance.visible = toonClearanceRef.current;
        const box = new THREE.Box3().setFromObject(trap);
        const c = box.getCenter(new THREE.Vector3());
        const cam = world.camera as OBC.SimpleCamera;
        const fov = (cam.three as THREE.PerspectiveCamera).fov ?? 50;
        const r = box.getSize(new THREE.Vector3()).length() * 0.55;
        const pos = c.clone().add(ISO.clone().multiplyScalar(r / Math.tan((fov * Math.PI) / 360)));
        await cam.controls.setLookAt(pos.x, pos.y, pos.z, c.x, c.y, c.z, false);
        setStatus("klaar");
      })
      .catch((e: unknown) => {
        if (!actief) return;
        setFout(e instanceof Error ? e.message : String(e));
        setStatus("fout");
      });

    return () => {
      actief = false;
      clearanceGroep.current = null;
      components.dispose();
    };
  }, [ifc]);

  useEffect(() => {
    if (clearanceGroep.current) clearanceGroep.current.visible = toonClearance;
  }, [toonClearance]);

  return (
    <div className="relative">
      <div
        ref={container}
        data-testid="trap-3d"
        data-status={status}
        className="h-[520px] w-full overflow-hidden rounded-md border border-[var(--oaec-border)]"
      />
      {status === "laden" && (
        <p role="status" className="absolute left-3 top-3 text-xs text-on-surface-secondary">
          {t("trap.drieD.laden")}
        </p>
      )}
      {fout && (
        <p role="alert" className="absolute left-3 top-3 text-xs text-[var(--theme-danger-color)]">
          {t("trap.drieD.fout", { fout })}
        </p>
      )}
    </div>
  );
}
