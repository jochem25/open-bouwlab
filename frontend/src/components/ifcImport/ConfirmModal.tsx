import Modal from "../Modal";

export interface ConfirmNotices {
  /** Blokkerende bevindingen (berichten) die voor de goedgekeurde ruimten gelden. */
  blocking: string[];
  /** Vlakken naar niet-overgenomen verwarmde ruimten (0 W/K). */
  orphanedHeated: number;
  /** Vlakken naar niet-overgenomen onverwarmde ruimten (f_k = 0,5). */
  orphanedUnheated: number;
  /** Aantal bestaande vertrekken dat wordt vervangen. */
  existingRoomCount?: number;
  /** Het project heeft al ruimten die worden vervangen. */
  replacesExisting: boolean;
  approvedCount: number;
}

interface Props {
  open: boolean;
  notices: ConfirmNotices;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmModal({ open, notices, onConfirm, onCancel }: Props) {
  const {
    blocking,
    orphanedHeated,
    orphanedUnheated,
    replacesExisting,
    approvedCount,
    existingRoomCount,
  } = notices;
  return (
    <Modal
      open={open}
      onClose={onCancel}
      title="Overnemen in project"
      width={520}
      footer={
        <>
          <button
            type="button"
            onClick={onCancel}
            className="rounded border border-[var(--oaec-border-subtle)] px-4 py-1.5 text-sm text-on-surface"
          >
            Annuleren
          </button>
          <button
            type="button"
            onClick={onConfirm}
            data-testid="confirm-import"
            className="rounded bg-primary px-4 py-1.5 text-sm font-medium text-white hover:bg-primary-hover"
          >
            {replacesExisting ? "Vervangen" : "Toch overnemen"}
          </button>
        </>
      }
    >
      <div className="flex flex-col gap-3 text-sm text-on-surface">
        <p>
          {approvedCount} {approvedCount === 1 ? "ruimte wordt" : "ruimten worden"} overgenomen.
        </p>
        {replacesExisting && (
          <p>
            Vervangt alle {existingRoomCount ?? "bestaande"} vertrekken; projectgegevens,
            klimaat en instellingen blijven behouden.
          </p>
        )}
        {blocking.length > 0 && (
          <div role="alert" className="rounded-md border border-red-400 px-3 py-2">
            <p className="font-medium text-red-400">
              Er zijn blokkerende bevindingen bij de goedgekeurde ruimten
            </p>
            <ul className="mt-1 list-disc pl-5 text-on-surface-secondary">
              {blocking.map((m, i) => (
                <li key={i}>{m}</li>
              ))}
            </ul>
          </div>
        )}
        {orphanedHeated > 0 && (
          <p>
            {orphanedHeated} vlakken grenzen aan niet-overgenomen verwarmde ruimten en tellen als
            0 W/K.
          </p>
        )}
        {orphanedUnheated > 0 && (
          <p>
            {orphanedUnheated} vlakken grenzen aan niet-overgenomen onverwarmde ruimten en rekenen
            met f_k = 0,5 (standaard).
          </p>
        )}
      </div>
    </Modal>
  );
}
