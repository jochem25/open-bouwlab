import { TRILLING_UIT_ZIN } from "./formulier";

/** Vaste, niet-sluitbare banner: de trillingstoets is door de gebruiker uitgezet. */
export function TrillingUitBanner() {
  return (
    <div
      role="note"
      data-testid="trilling-uit-banner"
      className="rounded-md border border-[var(--theme-warning-border)] bg-[var(--theme-warning-bg)] px-4 py-3 text-sm font-medium text-[var(--theme-warning-text)]"
    >
      {TRILLING_UIT_ZIN}
    </div>
  );
}
