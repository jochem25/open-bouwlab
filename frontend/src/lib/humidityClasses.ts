/**
 * Vochtklassen binnenklimaat voor de jaarlijkse vochtbalans
 * (NEN-EN-ISO 13788, bijlage A "Classes of internal humidity load", informatief).
 *
 * Methode (geverifieerd, ISO 13788:2001 par. 4.2 c, vergelijking 8):
 *   pi = pe + Δp, waarbij Δp volgens het gebruik van het gebouw wordt
 *   vermenigvuldigd met 1,10 als veiligheidsmarge.
 *
 * Δp-WAARDEN PER KLASSE: NOG TE VERIFIËREN tegen NEN-EN-ISO 13788 bijlage A.
 * De tabel hieronder is de indicatie uit opdracht O-2026-11-23-01: Δp bij
 * θe ≤ 0 °C = 270 / 540 / 810 / 1080 / 1350 Pa, lineair afnemend naar 0 bij
 * θe ≥ 20 °C. Bijlage A zelf is niet ingezien (niet in het normcorpus, niet in
 * de openbare preview). Pas de constanten hier aan zodra de norm is ingezien en
 * zet dan HUMIDITY_CLASS_TABLE_VERIFIED op true.
 */

// ---------- Tabel ----------

export type HumidityClass = 1 | 2 | 3 | 4 | 5;

export const HUMIDITY_CLASSES: readonly HumidityClass[] = [1, 2, 3, 4, 5];

/** Is de Δp-tabel geverifieerd tegen de normtekst? */
export const HUMIDITY_CLASS_TABLE_VERIFIED = false;

/** Tekst voor UI en rapport zolang de tabel niet geverifieerd is. */
export const HUMIDITY_CLASS_UNVERIFIED_NOTE =
  "Δp-waarden per vochtklasse nog te verifiëren tegen NEN-EN-ISO 13788 bijlage A.";

/** Veiligheidsfactor op Δp (ISO 13788:2001 par. 4.2 c). */
export const HUMIDITY_SAFETY_FACTOR = 1.1;

/** Buitentemperatuur waaronder Δp maximaal is [°C]. */
export const DELTA_P_THETA_LOW = 0;

/** Buitentemperatuur waarboven Δp nul is [°C]. */
export const DELTA_P_THETA_HIGH = 20;

/** Δp bij θe ≤ DELTA_P_THETA_LOW [Pa], per klasse. */
export const DELTA_P_MAX: Record<HumidityClass, number> = {
  1: 270,
  2: 540,
  3: 810,
  4: 1080,
  5: 1350,
};

export const HUMIDITY_CLASS_EXAMPLES: Record<HumidityClass, string> = {
  1: "opslag droge goederen",
  2: "kantoren, woningen met normale bezetting en ventilatie",
  3: "onbekend gebruik",
  4: "sporthallen, keukens, kantines",
  5: "wasserijen, brouwerijen, zwembaden",
};

/** Standaardklasse bij onbekend gebruik. */
export const DEFAULT_HUMIDITY_CLASS: HumidityClass = 3;

// ---------- Ruimtetypen (modus "type ruimte") ----------

export interface RoomTypeOption {
  id: string;
  label: string;
  humidityClass: HumidityClass;
}

export const ROOM_TYPE_OPTIONS: readonly RoomTypeOption[] = [
  { id: "opslag-droog", label: "Opslag droge goederen", humidityClass: 1 },
  { id: "kantoor", label: "Kantoor", humidityClass: 2 },
  { id: "woning", label: "Woning (normale bezetting en ventilatie)", humidityClass: 2 },
  { id: "onbekend", label: "Onbekend gebruik", humidityClass: 3 },
  { id: "sporthal", label: "Sporthal", humidityClass: 4 },
  { id: "keuken-kantine", label: "Keuken / kantine", humidityClass: 4 },
  { id: "wasserij", label: "Wasserij", humidityClass: 5 },
  { id: "brouwerij", label: "Brouwerij", humidityClass: 5 },
  { id: "zwembad", label: "Zwembad", humidityClass: 5 },
];

export const DEFAULT_ROOM_TYPE_ID = "onbekend";

export function getRoomType(id: string): RoomTypeOption | undefined {
  return ROOM_TYPE_OPTIONS.find((r) => r.id === id);
}

// ---------- Berekening ----------

/**
 * Dampdrukverschil binnen-buiten Δp [Pa] voor een vochtklasse bij
 * (maandgemiddelde) buitentemperatuur θe, ZONDER veiligheidsfactor.
 */
export function deltaPForClass(humidityClass: HumidityClass, thetaE: number): number {
  const dpMax = DELTA_P_MAX[humidityClass];
  if (thetaE <= DELTA_P_THETA_LOW) return dpMax;
  if (thetaE >= DELTA_P_THETA_HIGH) return 0;
  return dpMax * (DELTA_P_THETA_HIGH - thetaE) / (DELTA_P_THETA_HIGH - DELTA_P_THETA_LOW);
}

/** Dampdruk binnen pi = pe + 1,10·Δp [Pa]. */
export function internalVapourPressure(
  humidityClass: HumidityClass,
  thetaE: number,
  pe: number,
): number {
  return pe + HUMIDITY_SAFETY_FACTOR * deltaPForClass(humidityClass, thetaE);
}

/** Leesbare omschrijving van de Δp-tabel (UI en rapport). */
export function describeDeltaPTable(): string {
  const values = HUMIDITY_CLASSES.map((c) => `${DELTA_P_MAX[c]}`).join(" / ");
  return (
    `Δp bij θe ≤ ${DELTA_P_THETA_LOW} °C: ${values} Pa (klasse 1 t/m 5), lineair naar 0 bij ` +
    `θe ≥ ${DELTA_P_THETA_HIGH} °C; pi = pe + ${HUMIDITY_SAFETY_FACTOR.toFixed(2).replace(".", ",")}·Δp.`
  );
}
