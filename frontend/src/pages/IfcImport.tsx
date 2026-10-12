import { useCallback, useState } from "react";
import { useNavigate } from "react-router-dom";

import { applyThermalImport } from "../lib/applyThermalImport";
import { BengImportError, importBengDocument, type BengCheck } from "../lib/bengImport";
import {
  originDestinations,
  routeAfterImport,
  type ImportDestinationId,
} from "../lib/ifcImportDestinations";
import type { IfcImportBengOrigin, IfcImportOrigin } from "../types/ifcImport";
import {
  analyseIfc,
  stripAnalyse,
  type AnalyseProgress,
  type IfcAnalyseResponse,
} from "../lib/ifcAnalyse";
import type { FilteredImport } from "../lib/ifcImportFilter";
import { importThermal, type ThermalImportResult } from "../lib/thermalImport";
import { useModellerStore } from "../components/modeller/modellerStore";
import { PageHeader } from "../components/layout/PageHeader";
import { IfcImportChecklistPanel } from "../components/ifcImport/IfcImportChecklistPanel";
import { computeMaterialMatches, computeMaterialSuggestions } from "../lib/ifcImportChecklist";
import { applyRoomFunctions, computeFunctionSources } from "../lib/roomFunctionFromModel";
import { ImportResultView } from "../components/ifcImport/ImportResultView";
import { UploadPanel } from "../components/ifcImport/UploadPanel";
import { IfcImportOriginNote } from "../components/ifcImport/IfcImportOriginNote";
import { useProjectStore } from "../store/projectStore";
import { useToastStore } from "../store/toastStore";

interface Loaded {
  response: IfcAnalyseResponse;
  importResult: ThermalImportResult;
  /** Alleen paginastate, voor het originele model in de 3D-viewer (nooit in een store). */
  file: File | null;
}

/**
 * Tab "IFC-import" op projectniveau (route {@link IFC_IMPORT_ROUTE}): upload,
 * controleer, keur goed en neem over naar een of meer bestemmingen.
 */
export function IfcImport() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<AnalyseProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [withBeng, setWithBeng] = useState(false);
  const [bengCheck, setBengCheck] = useState<BengCheck | null>(null);
  const setBengGeometry = useProjectStore((s) => s.setBengGeometry);

  const existingRoomCount = useProjectStore((s) => s.project.rooms.length);
  const replaceRoomsFromImport = useProjectStore((s) => s.replaceRoomsFromImport);
  const setIfcImport = useProjectStore((s) => s.setIfcImport);
  const origin = useProjectStore((s) => s.ifcImport);
  const addToast = useToastStore((s) => s.addToast);
  const setImportedBoundaries = useModellerStore((s) => s.setImportedBoundaries);
  const setImportGeometry = useModellerStore((s) => s.setImportGeometry);
  const ensureProjectConstruction = useModellerStore((s) => s.ensureProjectConstruction);

  const handleFile = useCallback(async (file: File) => {
    setBusy(true);
    setError(null);
    setLoaded(null);
    setBengCheck(null);
    setProgress({ phase: "upload", fraction: 0 });
    try {
      const response = await analyseIfc(file, setProgress, { beng: withBeng });
      // Hergebruik de echte mapping + catalogus van de bestaande thermal-import.
      const importResult = await importThermal(response.thermal);
      setLoaded({ response, importResult, file });
      if (response.beng) {
        // Status (compleet/onvolledig, te controleren) bepaalt o.a. de begin-
        // weergave van de 3D-viewer; overnemen gebeurt pas op de knop.
        setBengCheck({ phase: "loading" });
        importBengDocument(response.beng).then(
          (result) => setBengCheck({ phase: "ready", result }),
          (e: unknown) =>
            setBengCheck({
              phase: "error",
              message: e instanceof BengImportError || e instanceof Error ? e.message : String(e),
            }),
        );
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "IFC-analyse mislukt");
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }, [withBeng]);



  const handleImport = useCallback(
    (filtered: FilteredImport, approvedIds: string[], destinations: ImportDestinationId[]) => {
      if (!loaded) return;
      const chosen = new Set(destinations);
      const bengResult = bengCheck?.phase === "ready" ? bengCheck.result : null;
      const bengGeometry = chosen.has("beng") ? bengResult?.beng_geometry : undefined;
      if (
        bengGeometry &&
        useProjectStore.getState().bengGeometry &&
        !window.confirm("De BENG-geometrie van dit project vervangen door die uit het IFC?")
      ) {
        return;
      }
      const bengOrigin: IfcImportBengOrigin | undefined =
        bengGeometry && bengResult
          ? {
              imported_at: new Date().toISOString(),
              source: bengResult.source,
              status: bengResult.status,
              te_controleren: bengResult.te_controleren,
              zones: bengGeometry.zones?.length ?? 0,
              vlakken: (bengGeometry.zones ?? []).reduce((n, z) => n + (z.gevels?.length ?? 0), 0),
            }
          : undefined;
      const messages: string[] = [];
      if (bengGeometry) {
        setBengGeometry(bengGeometry);
        messages.push("BENG-geometrie (te controleren)");
      }
      if (!chosen.has("warmteverlies")) {
        // Alleen BENG: de vertrekken en de warmteverlies-herkomst blijven staan.
        const prev = useProjectStore.getState().ifcImport;
        const merged: IfcImportOrigin = prev
          ? {
              ...prev,
              destinations: [...new Set([...originDestinations(prev), "beng"])],
              ...(bengOrigin ? { beng: bengOrigin } : {}),
            }
          : {
              ifc_filename: loaded.response.source_filename,
              engine_name: loaded.response.engine.name,
              engine_version: loaded.response.engine.version,
              imported_at: new Date().toISOString(),
              rooms_count: 0,
              approved_room_ids: [],
              destinations: ["beng"],
              ...(bengOrigin ? { beng: bengOrigin } : {}),
            };
        setIfcImport(merged);
        addToast(`${messages.join(", ")} overgenomen uit ${loaded.response.source_filename}`, "success");
        navigate(routeAfterImport(chosen));
        return;
      }
      const { importFile, importResult } = filtered;
      const applied = applyThermalImport(
        importFile,
        importResult,
        importFile.rooms.map((r) => ({ ...r })),
        (importFile.openings ?? []).map((o) => ({ ...o })),
        new Map(),
        {
          ensureProjectConstruction,
          getProjectConstructions: () => useModellerStore.getState().projectConstructions,
        },
      );
      // Alleen in dit overnamepad: functie uit model, naam, default (wizard blijft ongemoeid).
      const project = applyRoomFunctions(applied.project, importFile.rooms, loaded.response.rooms_extra);
      // Alleen de ruimten vervangen: projectgegevens, klimaat, instellingen en
      // serverkoppeling blijven behouden.
      replaceRoomsFromImport(project);
      setImportedBoundaries(applied.boundaries);
      setImportGeometry(applied.importGeometry);
      // Defaults uit het BEHOUDEN project (niet uit de mapper-defaults).
      const kept = useProjectStore.getState().project;
      setIfcImport({
        ifc_filename: loaded.response.source_filename,
        engine_name: loaded.response.engine.name,
        engine_version: loaded.response.engine.version,
        imported_at: new Date().toISOString(),
        rooms_count: approvedIds.length,
        approved_room_ids: approvedIds,
        room_function_sources: computeFunctionSources(
          project,
          importFile.rooms,
          loaded.response.rooms_extra,
          approvedIds,
        ),
        material_matches: computeMaterialMatches(loaded.response.thermal, approvedIds),
        material_suggestions: computeMaterialSuggestions(loaded.response.report),
        defaults: {
          heating_system: kept.rooms[0]?.heating_system ?? "",
          ventilation_system: kept.ventilation.system_type,
          theta_e: kept.climate.theta_e ?? 0,
          qv10: kept.building.qv10,
          building_type: kept.building.building_type,
        },
        analyse: stripAnalyse(loaded.response),
        destinations,
        ...(bengOrigin ? { beng: bengOrigin } : {}),
      });
      messages.unshift(`${approvedIds.length} ruimten`);
      addToast(`${messages.join(" en ")} overgenomen uit ${loaded.response.source_filename}`, "success");
      navigate(routeAfterImport(chosen));
    },
    [
      loaded,
      bengCheck,
      setBengGeometry,
      ensureProjectConstruction,
      replaceRoomsFromImport,
      setImportedBoundaries,
      setImportGeometry,
      setIfcImport,
      addToast,
      navigate,
    ],
  );

  return (
    <div>
      <PageHeader
        title="IFC-import"
        subtitle="Upload een IFC, controleer de ruimten en neem ze over"
      />
      <div className="flex flex-col gap-5 p-6">
        {/* Herkomst/checklist van een eerdere import verwarren naast een nieuwe,
            nog niet overgenomen analyse: pas tonen als die is overgenomen of gewist. */}
        {!loaded && origin && <IfcImportOriginNote origin={origin} />}
        {!loaded && <IfcImportChecklistPanel />}
        <UploadPanel
          busy={busy}
          progress={progress}
          error={error}
          onFile={handleFile}
          beng={withBeng}
          onBengChange={setWithBeng}
        />
        {loaded && (
          <ImportResultView
            key={loaded.response.source_filename + loaded.response.thermal.exported_at}
            response={loaded.response}
            importResult={loaded.importResult}
            ifcFile={loaded.file}
            existingRoomCount={existingRoomCount}
            onImport={handleImport}
            bengCheck={bengCheck}
          />
        )}
      </div>
    </div>
  );
}
