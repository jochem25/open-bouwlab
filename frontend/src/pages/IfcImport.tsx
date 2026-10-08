import { useCallback, useState } from "react";
import { useNavigate } from "react-router-dom";

import { applyThermalImport } from "../lib/applyThermalImport";
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
import {
  DISMISS_KEY,
  IfcImportChecklistPanel,
} from "../components/ifcImport/IfcImportChecklistPanel";
import { applyRoomFunctionsFromNames } from "../lib/roomFunctionFromName";
import { ImportResultView } from "../components/ifcImport/ImportResultView";
import { UploadPanel } from "../components/ifcImport/UploadPanel";
import { IfcImportOriginNote } from "../components/ifcImport/IfcImportOriginNote";
import { useProjectStore } from "../store/projectStore";
import { useToastStore } from "../store/toastStore";

interface Loaded {
  response: IfcAnalyseResponse;
  importResult: ThermalImportResult;
}

/** Tab "IFC-import" (route `/ifc-import`): upload, controleer, keur goed, neem over. */
export function IfcImport() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<AnalyseProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState<Loaded | null>(null);

  const existingRoomCount = useProjectStore((s) => s.project.rooms.length);
  const setProject = useProjectStore((s) => s.setProject);
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
    setProgress({ phase: "upload", fraction: 0 });
    try {
      const response = await analyseIfc(file, setProgress);
      // Hergebruik de echte mapping + catalogus van de bestaande thermal-import.
      const importResult = await importThermal(response.thermal);
      setLoaded({ response, importResult });
    } catch (e) {
      setError(e instanceof Error ? e.message : "IFC-analyse mislukt");
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }, []);

  const handleImport = useCallback(
    (filtered: FilteredImport, approvedIds: string[]) => {
      if (!loaded) return;
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
      // Alleen in dit overnamepad: functie uit de naam (wizard blijft ongemoeid).
      const project = applyRoomFunctionsFromNames(applied.project, importFile.rooms);
      setProject(project);
      setImportedBoundaries(applied.boundaries);
      setImportGeometry(applied.importGeometry);
      // Na setProject: die wist de herkomst van het vorige project.
      setIfcImport({
        ifc_filename: loaded.response.source_filename,
        engine_name: loaded.response.engine.name,
        engine_version: loaded.response.engine.version,
        imported_at: new Date().toISOString(),
        rooms_count: approvedIds.length,
        approved_room_ids: approvedIds,
        defaults: {
          heating_system: project.rooms[0]?.heating_system ?? "",
          ventilation_system: project.ventilation.system_type,
          theta_e: project.climate.theta_e ?? 0,
          qv10: project.building.qv10,
          building_type: project.building.building_type,
        },
        analyse: stripAnalyse(loaded.response),
      });
      try {
        sessionStorage.removeItem(DISMISS_KEY);
      } catch {
        // sessionStorage niet beschikbaar
      }
      addToast(`${approvedIds.length} ruimten overgenomen uit ${loaded.response.source_filename}`, "success");
      navigate("/rooms");
    },
    [
      loaded,
      ensureProjectConstruction,
      setProject,
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
        {origin && <IfcImportOriginNote origin={origin} />}
        <IfcImportChecklistPanel />
        <UploadPanel busy={busy} progress={progress} error={error} onFile={handleFile} />
        {loaded && (
          <ImportResultView
            key={loaded.response.source_filename + loaded.response.thermal.exported_at}
            response={loaded.response}
            importResult={loaded.importResult}
            existingRoomCount={existingRoomCount}
            onImport={handleImport}
          />
        )}
      </div>
    </div>
  );
}
