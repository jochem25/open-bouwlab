import { useRef, useState } from "react";

import { IFC_MAX_BYTES, IFC_MAX_MB_LABEL, type AnalyseProgress } from "../../lib/ifcAnalyse";
import { Card } from "../ui/Card";

const ACCEPT = ".ifc,.ifczip";

/** Client-side validatie; geeft een NL-foutmelding of null. */
export function validateIfcFile(file: { name: string; size: number }): string | null {
  const name = file.name.toLowerCase();
  if (!name.endsWith(".ifc") && !name.endsWith(".ifczip")) {
    return "Kies een .ifc- of .ifczip-bestand.";
  }
  if (file.size > IFC_MAX_BYTES) {
    return `Het bestand is te groot (maximaal ${IFC_MAX_MB_LABEL}).`;
  }
  return null;
}

export function progressLabel(p: AnalyseProgress | null): string {
  if (!p) return "";
  if (p.phase === "analyse") return "Analyseren… een groot model kan ongeveer een minuut duren.";
  return `Uploaden… ${Math.round(p.fraction * 100)}%`;
}

interface Props {
  busy: boolean;
  progress: AnalyseProgress | null;
  error: string | null;
  onFile: (file: File) => void;
}

export function UploadPanel({ busy, progress, error, onFile }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  const accept = (file: File | undefined) => {
    if (!file) return;
    const problem = validateIfcFile(file);
    setLocalError(problem);
    if (!problem) onFile(file);
  };

  const shownError = localError ?? error;

  return (
    <Card title="IFC-bestand">
      <div
        role="button"
        tabIndex={0}
        data-testid="ifc-dropzone"
        onClick={() => !busy && inputRef.current?.click()}
        onKeyDown={(e) => e.key === "Enter" && !busy && inputRef.current?.click()}
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          if (!busy) accept(e.dataTransfer.files[0]);
        }}
        className={`flex cursor-pointer flex-col items-center gap-1 rounded-lg border-2 border-dashed px-6 py-10 text-center ${
          dragOver ? "border-primary bg-primary/10" : "border-[var(--oaec-border-subtle)]"
        } ${busy ? "cursor-wait opacity-60" : ""}`}
      >
        <p className="text-sm font-medium text-on-surface">
          Sleep een IFC hierheen of klik om te kiezen
        </p>
        <p className="text-xs text-on-surface-muted">.ifc of .ifczip, max {IFC_MAX_MB_LABEL}</p>
        <input
          ref={inputRef}
          type="file"
          accept={ACCEPT}
          className="hidden"
          data-testid="ifc-file-input"
          onChange={(e) => {
            accept(e.target.files?.[0]);
            e.target.value = "";
          }}
        />
      </div>
      {busy && (
        <div className="mt-4" role="status" aria-live="polite">
          <p className="text-sm text-on-surface-secondary">{progressLabel(progress)}</p>
          {progress?.phase === "upload" && (
            <div className="mt-2 h-1.5 w-full overflow-hidden rounded bg-[var(--oaec-border-subtle)]">
              <div
                className="h-full bg-primary"
                style={{ width: `${Math.round(progress.fraction * 100)}%` }}
              />
            </div>
          )}
        </div>
      )}
      {shownError && (
        <p
          role="alert"
          className="mt-4 rounded-md border border-red-400 px-3 py-2 text-sm text-red-400"
        >
          {shownError}
        </p>
      )}
    </Card>
  );
}
