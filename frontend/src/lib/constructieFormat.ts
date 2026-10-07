/**
 * Weergave-formattering voor de constructiemodule.
 * Alleen voor weergave; beslissingen gebeuren op de onafgeronde waarde.
 */

/** Vaste banner-tekst; gelijk aan de disclaimer uit de rekenkern. */
export const CONSTRUCTIE_DISCLAIMER =
  "Indicatieve voordimensionering - vervangt geen constructieve berekening. Definitieve afmetingen, verbindingen en opleggingen worden door de constructeur bepaald.";

/** Rond af op `decimalen`, half naar boven (symmetrisch rond nul), met float-ruiscorrectie. */
export function rondHalfOp(x: number, decimalen: number): number {
  const f = 10 ** decimalen;
  const afgerond = Math.round(Math.abs(x) * f + 1e-9) / f;
  return x < 0 ? -afgerond : afgerond;
}

/** Getal met `decimalen` decimalen en Nederlandse komma (half naar boven). */
export function formatGetal(x: number | null | undefined, decimalen: number): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return "-";
  return rondHalfOp(x, decimalen).toFixed(decimalen).replace(".", ",");
}

/** Unity check: 2 decimalen, NL-komma, half naar boven. */
export function formatUc(uc: number | null | undefined): string {
  return formatGetal(uc, 2);
}
