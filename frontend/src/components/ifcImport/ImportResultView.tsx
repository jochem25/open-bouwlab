import { useMemo, useRef, useState } from "react";

import type { IfcAnalyseResponse, QcFinding } from "../../lib/ifcAnalyse";
import { filterApprovedRooms, type FilteredImport } from "../../lib/ifcImportFilter";
import {
  ORIENTATION_LABEL,
  adjacentLabel,
  defaultApproved,
  envelopeArea,
  findingsForRoom,
  openingArea,
  realRooms,
  roomSurfaces,
  roomVolume,
  totalFloorArea,
} from "../../lib/ifcImportView";
import type { ThermalImportResult, ThermalRoom } from "../../lib/thermalImport";
import { qcCodeText } from "../../lib/qcCodeTexts";
import { openingTypeLabel } from "../../lib/thermalImport";
import { Card } from "../ui/Card";
import { ConfirmModal } from "./ConfirmModal";

const fmt = (n: number | undefined, digits = 1): string =>
  n == null ? "-" : n.toLocaleString("nl-NL", { minimumFractionDigits: digits, maximumFractionDigits: digits });

const VERDICT_STYLE: Record<string, string> = {
  ok: "border-green-500 text-green-500",
  waarschuwing: "border-amber-500 text-amber-500",
  blokkerend: "border-red-400 text-red-400",
};
const VERDICT_LABEL: Record<string, string> = {
  ok: "Bevindingen: in orde",
  waarschuwing: "Bevindingen: waarschuwingen",
  blokkerend: "Bevindingen: blokkerend",
};

const ENVELOPE_HINT =
  "Som van het bruto oppervlak van alle vlakken naar buiten, grond, water en onverwarmde ruimten.";

interface Props {
  response: IfcAnalyseResponse;
  importResult: ThermalImportResult;
  /** Aantal ruimten dat het project nu al heeft (bepaalt "Vervangen"). */
  existingRoomCount: number;
  onImport: (filtered: FilteredImport, approvedIds: string[]) => void;
}

export function ImportResultView({ response, importResult, existingRoomCount, onImport }: Props) {
  const { thermal, qc } = response;
  const rooms = useMemo(() => realRooms(thermal), [thermal]);
  const [approved, setApproved] = useState<Set<string>>(() =>
    defaultApproved(thermal, qc.findings),
  );
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const rowRefs = useRef<Map<string, HTMLElement>>(new Map());

  const blockingFindings = qc.findings.filter((f) => f.severity === "blocking");
  const warningFindings = qc.findings.filter((f) => f.severity === "warning");
  const infoFindings = qc.findings.filter((f) => f.severity === "info");
  const numberById = useMemo(
    () => new Map((response.rooms_extra ?? []).map((r) => [r.room_id, r.number])),
    [response.rooms_extra],
  );

  const toggleApproved = (id: string) =>
    setApproved((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const allApproved = rooms.length > 0 && rooms.every((r) => approved.has(r.id));
  const toggleAll = () =>
    setApproved(allApproved ? new Set() : new Set(rooms.map((r) => r.id)));

  const toggleExpanded = (id: string, forceOpen = false) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (forceOpen || !next.has(id)) next.add(id);
      else next.delete(id);
      return next;
    });

  const jumpToRoom = (id: string) => {
    toggleExpanded(id, true);
    rowRefs.current.get(id)?.scrollIntoView?.({ behavior: "smooth", block: "center" });
  };

  const filtered = useMemo(
    () => filterApprovedRooms(thermal, importResult, approved),
    [thermal, importResult, approved],
  );
  const relevantBlocking = blockingFindings.filter(
    (f) => !f.room_id || approved.has(f.room_id),
  );
  const needsConfirm =
    relevantBlocking.length > 0 ||
    filtered.orphanedHeatedCount > 0 ||
    filtered.orphanedUnheatedCount > 0 ||
    existingRoomCount > 0;

  const handleImportClick = () => {
    if (needsConfirm) setConfirming(true);
    else onImport(filtered, [...approved]);
  };

  return (
    <div className="flex flex-col gap-5">
      <Card title="Samenvatting">
        <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm md:grid-cols-4">
          <Stat label="Bestand" value={response.source_filename} />
          <Stat label="Engine" value={`${response.engine.name} ${response.engine.version}`} />
          <Stat label="Ruimten" value={String(rooms.length)} testId="stat-rooms" />
          <Stat label="Vloeroppervlak" value={`${fmt(totalFloorArea(rooms))} m²`} />
          <Stat
            label="Schilsom"
            value={`${fmt(envelopeArea(thermal))} m²`}
            title={ENVELOPE_HINT}
          />
        </dl>
        <p
          data-testid="verdict-badge"
          className={`mt-4 inline-block rounded-full border px-3 py-1 text-xs font-medium ${VERDICT_STYLE[qc.verdict] ?? ""}`}
        >
          {VERDICT_LABEL[qc.verdict] ?? qc.verdict}
        </p>
        <p className="mt-2 text-sm text-on-surface-secondary" data-testid="modelcheck">
          Modelcheck: {qc.modelcheck?.verdict ?? "-"}
          {(qc.modelcheck?.failed_rules.length ?? 0) > 0 &&
            ` (niet gehaald: ${qc.modelcheck?.failed_rules.join(", ")})`}
        </p>
        <p className="mt-1 text-sm text-on-surface-secondary" data-testid="qc-counts">
          Blokkerend {qc.counts?.blocking ?? blockingFindings.length} · Waarschuwingen{" "}
          {qc.counts?.warning ?? warningFindings.length} · Info{" "}
          {qc.counts?.info ?? infoFindings.length}
        </p>
        <FindingGroups
          title="Blokkerend"
          tone="text-red-400"
          findings={blockingFindings}
          rooms={thermal.rooms}
          onJump={jumpToRoom}
        />
        <FindingGroups
          title="Waarschuwingen"
          tone="text-amber-500"
          collapsible
          findings={warningFindings}
          rooms={thermal.rooms}
          onJump={jumpToRoom}
        />
        <FindingGroups
          title="Info"
          tone="text-on-surface-secondary"
          collapsible
          previewGroups={0}
          findings={infoFindings}
          rooms={thermal.rooms}
          onJump={jumpToRoom}
        />
      </Card>

      <Card title="Ruimten">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm" data-testid="rooms-table">
            <thead className="text-xs text-on-surface-muted">
              <tr>
                <th className="py-1 pr-2">
                  <input
                    type="checkbox"
                    aria-label="Alles goedkeuren"
                    checked={allApproved}
                    onChange={toggleAll}
                  />
                </th>
                <th className="pr-3">Nr.</th>
                <th className="pr-3">Naam</th>
                <th className="pr-3">Verdieping</th>
                <th className="pr-3 text-right">Vloer m²</th>
                <th className="pr-3 text-right">Hoogte m</th>
                <th className="pr-3 text-right">Volume m³</th>
                <th className="pr-3">Verwarmd</th>
                <th className="pr-3 text-right">QC</th>
              </tr>
            </thead>
            <tbody>
              {rooms.map((room) => (
                <RoomRows
                  key={room.id}
                  room={room}
                  response={response}
                  checked={approved.has(room.id)}
                  open={expanded.has(room.id)}
                  onCheck={() => toggleApproved(room.id)}
                  onToggle={() => toggleExpanded(room.id)}
                  number={numberById.get(room.id) ?? null}
                  register={(el) => {
                    if (el) rowRefs.current.set(room.id, el);
                  }}
                />
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <Card title="Constructies">
        <table className="w-full text-left text-sm" data-testid="constructions-table">
          <thead className="text-xs text-on-surface-muted">
            <tr>
              <th className="py-1 pr-3">Beschrijving</th>
              <th className="pr-3">Lagen</th>
              <th className="pr-3 text-right">Vlakken</th>
              <th className="text-right">m²</th>
            </tr>
          </thead>
          <tbody>
            {importResult.construction_catalog.map((e) => (
              <tr key={e.id} className="border-t border-[var(--oaec-border-subtle)] align-top">
                <td className="py-1 pr-3 text-on-surface">{e.description}</td>
                <td className="pr-3 text-on-surface-secondary">
                  {e.layers.length === 0
                    ? "-"
                    : e.layers.map((l) => `${l.material} ${fmt(l.thickness_mm, 0)} mm`).join(", ")}
                </td>
                <td className="pr-3 text-right">{e.surface_count}</td>
                <td className="text-right">{fmt(e.total_area_m2)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      <div className="flex items-center gap-4">
        <button
          type="button"
          disabled={approved.size === 0}
          onClick={handleImportClick}
          data-testid="import-button"
          className="rounded bg-primary px-5 py-2 text-sm font-medium text-white hover:bg-primary-hover disabled:cursor-not-allowed disabled:opacity-40"
        >
          Overnemen in project
        </button>
        <span className="text-sm text-on-surface-secondary">
          {approved.size} van {rooms.length} ruimten goedgekeurd
        </span>
      </div>

      <ConfirmModal
        open={confirming}
        notices={{
          blocking: relevantBlocking.map((f) => f.message),
          orphanedHeated: filtered.orphanedHeatedCount,
          orphanedUnheated: filtered.orphanedUnheatedCount,
          existingRoomCount,
          replacesExisting: existingRoomCount > 0,
          approvedCount: approved.size,
        }}
        onCancel={() => setConfirming(false)}
        onConfirm={() => {
          setConfirming(false);
          onImport(filtered, [...approved]);
        }}
      />
    </div>
  );
}

function Stat({
  label,
  value,
  title,
  testId,
}: {
  label: string;
  value: string;
  title?: string;
  testId?: string;
}) {
  return (
    <div title={title}>
      <dt className="text-xs text-on-surface-muted">{label}</dt>
      <dd className="font-medium text-on-surface" data-testid={testId}>
        {value}
      </dd>
    </div>
  );
}

const COLLAPSED_PREVIEW_GROUPS = 3;

export interface FindingGroup {
  code: string;
  /** Som van `count` (een bevinding zonder count telt als 1). */
  count: number;
  modellerAction: boolean;
  message: string;
  roomIds: string[];
}

/** Groepeer bevindingen per code, in volgorde van eerste voorkomen. */
export function groupFindings(findings: QcFinding[]): FindingGroup[] {
  const groups = new Map<string, FindingGroup>();
  for (const f of findings) {
    const g = groups.get(f.code) ?? {
      code: f.code,
      count: 0,
      modellerAction: false,
      message: f.message,
      roomIds: [],
    };
    g.count += f.count ?? 1;
    g.modellerAction = g.modellerAction || f.modeller_action;
    if (f.room_id && !g.roomIds.includes(f.room_id)) g.roomIds.push(f.room_id);
    groups.set(f.code, g);
  }
  return [...groups.values()];
}

function FindingGroups({
  title,
  tone,
  findings,
  rooms,
  onJump,
  collapsible = false,
  previewGroups = COLLAPSED_PREVIEW_GROUPS,
}: {
  title: string;
  tone: string;
  collapsible?: boolean;
  /** Aantal groepen dat zichtbaar is zolang de lijst dicht staat. */
  previewGroups?: number;
  findings: QcFinding[];
  rooms: ThermalRoom[];
  onJump: (roomId: string) => void;
}) {
  const [showAll, setShowAll] = useState(!collapsible);
  if (findings.length === 0) return null;
  const groups = groupFindings(findings);
  const shown = showAll ? groups : groups.slice(0, previewGroups);
  const roomName = (id: string) => rooms.find((r) => r.id === id)?.name;
  return (
    <div className="mt-4" data-testid={`findings-${title.toLowerCase()}`}>
      <h4 className={`text-sm font-medium ${tone}`}>
        {title} ({findings.length})
      </h4>
      <ul className="mt-1 flex flex-col gap-1 text-sm text-on-surface-secondary">
        {shown.map((g) => (
          <li key={g.code}>
            <span className="mr-1 font-mono text-xs text-on-surface-muted">{g.code}</span>
            {g.count > 1 && <span className="mr-1">×{g.count}</span>}
            {g.modellerAction && (
              <span className="mr-1 rounded-full border border-[var(--oaec-border-subtle)] px-2 text-xs">
                modelleur
              </span>
            )}
            {qcCodeText(g.code, g.message)}
            {g.roomIds.map((id) => (
              <button
                key={id}
                type="button"
                className="ml-2 underline"
                onClick={() => onJump(id)}
              >
                {roomName(id) ?? id}
              </button>
            ))}
          </li>
        ))}
      </ul>
      {collapsible && groups.length > previewGroups && (
        <button
          type="button"
          className="mt-1 text-xs underline"
          aria-expanded={showAll}
          onClick={() => setShowAll((v) => !v)}
        >
          {showAll ? "Toon minder" : `Toon alle ${groups.length}`}
        </button>
      )}
    </div>
  );
}

function RoomRows({
  room,
  response,
  checked,
  open,
  onCheck,
  onToggle,
  register,
  number,
}: {
  number: string | null;
  room: ThermalRoom;
  response: IfcAnalyseResponse;
  checked: boolean;
  open: boolean;
  onCheck: () => void;
  onToggle: () => void;
  register: (el: HTMLElement | null) => void;
}) {
  const findings = findingsForRoom(response.qc.findings, room.id);
  const blocking = findings.some((f) => f.severity === "blocking");
  return (
    <>
      <tr
        ref={register}
        className="border-t border-[var(--oaec-border-subtle)]"
        data-testid="room-row"
      >
        <td className="py-1 pr-2">
          <input
            type="checkbox"
            aria-label={`Goedkeuren ${room.name}`}
            checked={checked}
            onChange={onCheck}
          />
        </td>
        <td className="pr-3 text-on-surface-muted">{number ?? "-"}</td>
        <td className="pr-3">
          <button type="button" className="text-left text-on-surface underline-offset-2 hover:underline" onClick={onToggle} aria-expanded={open}>
            {open ? "▾" : "▸"} {room.name}
          </button>
        </td>
        <td className="pr-3">{room.level ?? "-"}</td>
        <td className="pr-3 text-right">{fmt(room.area_m2)}</td>
        <td className="pr-3 text-right">{fmt(room.height_m, 2)}</td>
        <td className="pr-3 text-right">{fmt(roomVolume(room))}</td>
        <td className="pr-3">{room.type === "heated" ? "Ja" : "Nee"}</td>
        <td className={`pr-3 text-right ${blocking ? "text-red-400" : ""}`}>{findings.length}</td>
      </tr>
      {open && (
        <tr>
          <td />
          <td colSpan={8} className="pb-3">
            <RoomDetail room={room} response={response} />
          </td>
        </tr>
      )}
    </>
  );
}

function RoomDetail({ room, response }: { room: ThermalRoom; response: IfcAnalyseResponse }) {
  const { thermal } = response;
  const surfaces = roomSurfaces(thermal, room.id);
  if (surfaces.length === 0) {
    return <p className="text-sm text-on-surface-muted">Geen grensvlakken.</p>;
  }
  return (
    <table className="w-full text-left text-xs" data-testid="surface-table">
      <thead className="text-on-surface-muted">
        <tr>
          <th className="pr-3">Vlak</th>
          <th className="pr-3 text-right">m²</th>
          <th className="pr-3">Richting</th>
          <th className="pr-3">Grenst aan</th>
          <th>Constructie</th>
        </tr>
      </thead>
      <tbody>
        {surfaces.map((s) => (
          <tr key={s.construction.id} className="align-top">
            <td className="pr-3">{ORIENTATION_LABEL[s.orientation]}</td>
            <td className="pr-3 text-right">{fmt(s.construction.gross_area_m2, 2)}</td>
            <td className="pr-3">{s.construction.compass ?? "-"}</td>
            <td className="pr-3">{adjacentLabel(thermal, s.otherRoomId)}</td>
            <td>
              {s.construction.revit_type_name ?? "-"}
              {(s.construction.layers ?? []).length > 0 && (
                <span className="text-on-surface-muted">
                  {" "}
                  ({(s.construction.layers ?? [])
                    .map((l) => `${l.material} ${fmt(l.thickness_mm, 0)} mm`)
                    .join(", ")}
                  )
                </span>
              )}
              {s.openings.map((o) => (
                <div key={o.id} className="text-on-surface-secondary">
                  {openingTypeLabel(o.type)} {fmt(openingArea(o), 2)} m²
                </div>
              ))}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
