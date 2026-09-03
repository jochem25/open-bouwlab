#!/usr/bin/env node
/**
 * Genereert `materialen_database.json` (pyRevit-formaat) uit de RAW_MATERIALS
 * die in `frontend/src/lib/materialsDatabase.ts` staan gedefinieerd.
 *
 * `materialsDatabase.ts` is de enige bron voor materiaaldata (besluit
 * 2026-09-03, zie open-bouwlab/TODO.md). Het gegenereerde JSON-bestand mag
 * NIET handmatig bewerkt worden — wijzigingen horen in `materialsDatabase.ts`,
 * gevolgd door een regeneratie.
 *
 * Dit script parseert de TS-broncode als tekst (niet als module-import): elk
 * RAW_MATERIALS-item staat op één regel als een plat JS object-literal, en
 * alleen zo blijft het rijkere `categorie`-veld (bv. "Folie - Miofol") intact
 * — de geëxporteerde `MATERIALS_DATABASE` in datzelfde bestand mapt dat veld
 * juist samen tot de grovere `MaterialCategory`-enum, wat voor dit
 * pyRevit-schema te grof is.
 *
 * Gebruik (vanuit frontend/):
 *   npm run generate:materialen                              # print naar stdout
 *   npm run generate:materialen -- --out <pad-naar-json>      # schrijf naar bestand
 *
 * Cross-repo doel (bewust NIET hardcoded — dit is een publieke repo):
 *   npm run generate:materialen -- --out ../../../revit/pyrevit/extensions/bouwkunde.extension/lib/materialen_database.json
 */

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const DEFAULT_SOURCE = resolve(__dirname, "../src/lib/materialsDatabase.ts");

const ARRAY_START_MARKER = "const RAW_MATERIALS: RawMaterial[] = [";
const ARRAY_END_MARKER = "\n];";

const CORE_FIELDS = ["id", "categorie", "naam", "lambda", "mu", "rho", "rd_vast", "keywords"];
const ADDITIVE_FIELDS = ["lambda_nat", "sd_vast", "alpha", "merk", "hatch_pattern"];

/**
 * Parseert de RAW_MATERIALS-array uit de TS-broncode van materialsDatabase.ts.
 * Elk materiaal moet als één regel `{ id: ..., ... },` genoteerd staan — dat
 * is de bestaande conventie in het bronbestand (geverifieerd: elke regel die
 * met `{ id:` begint is precies één materiaal, 157 stuks).
 *
 * @param {string} tsSource
 * @returns {Record<string, unknown>[]}
 */
export function parseRawMaterials(tsSource) {
  const startIdx = tsSource.indexOf(ARRAY_START_MARKER);
  if (startIdx === -1) {
    throw new Error(
      "RAW_MATERIALS-array niet gevonden in materialsDatabase.ts — is de declaratie hernoemd of van vorm veranderd?",
    );
  }
  const bodyStart = startIdx + ARRAY_START_MARKER.length;
  const bodyEnd = tsSource.indexOf(ARRAY_END_MARKER, bodyStart);
  if (bodyEnd === -1) {
    throw new Error("Einde van de RAW_MATERIALS-array (`\\n];`) niet gevonden.");
  }

  const body = tsSource.slice(bodyStart, bodyEnd);
  const materials = [];

  for (const rawLine of body.split("\n")) {
    const line = rawLine.trim();
    if (line === "" || line.startsWith("//")) continue;
    if (!line.startsWith("{")) {
      throw new Error(
        `Onverwachte regel in RAW_MATERIALS (verwacht een object-literal op één regel): "${line}"`,
      );
    }

    const literal = line.endsWith(",") ? line.slice(0, -1) : line;
    let material;
    try {
      // Regel is een plat JS object-literal (strings/getallen/null/arrays) —
      // geen TS-specifieke syntax, dus veilig evalueerbaar als expressie.
      // eslint-disable-next-line no-new-func
      material = new Function(`"use strict"; return (${literal});`)();
    } catch (err) {
      throw new Error(`Kon materiaal-object niet parsen: "${literal}" (${err.message})`);
    }

    if (!material || typeof material !== "object") {
      throw new Error(`Materiaal-regel levert geen object op: "${literal}"`);
    }
    if (!material.id || typeof material.id !== "string") {
      throw new Error(`Materiaal zonder expliciet id: "${literal}"`);
    }
    materials.push(material);
  }

  return materials;
}

/** Escaped een string naar ASCII-veilige JSON (non-ASCII als \uXXXX, zoals Python json.dumps(ensure_ascii=True)). */
function escapeAsciiString(str) {
  let out = "";
  for (const ch of str) {
    if (ch === '"') {
      out += '\\"';
      continue;
    }
    if (ch === "\\") {
      out += "\\\\";
      continue;
    }
    if (ch === "\n") {
      out += "\\n";
      continue;
    }
    if (ch === "\r") {
      out += "\\r";
      continue;
    }
    if (ch === "\t") {
      out += "\\t";
      continue;
    }
    const cp = ch.codePointAt(0);
    if (cp < 0x20 || cp > 0x7e) {
      if (cp > 0xffff) {
        const c = cp - 0x10000;
        const hi = 0xd800 + (c >> 10);
        const lo = 0xdc00 + (c & 0x3ff);
        out += `\\u${hi.toString(16).padStart(4, "0")}\\u${lo.toString(16).padStart(4, "0")}`;
      } else {
        out += `\\u${cp.toString(16).padStart(4, "0")}`;
      }
    } else {
      out += ch;
    }
  }
  return `"${out}"`;
}

function serializeValue(value) {
  if (value === null || value === undefined) return "null";
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error(`Niet-eindig getal kan niet als JSON worden geserialiseerd: ${value}`);
    }
    return String(value);
  }
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "string") return escapeAsciiString(value);
  if (Array.isArray(value)) {
    return `[${value.map(serializeValue).join(", ")}]`;
  }
  throw new Error(`Onbekend waardetype in materiaaldata: ${JSON.stringify(value)}`);
}

/** Bouwt de sleutelvolgorde voor één materiaal-entry: kernvelden altijd, additieve velden alleen waar aanwezig in de bron. */
function orderedEntries(raw) {
  const entries = [];
  for (const key of CORE_FIELDS) {
    if (!(key in raw)) {
      throw new Error(`Materiaal "${raw.id}" mist verplicht veld "${key}"`);
    }
    entries.push([key, raw[key]]);
  }
  for (const key of ADDITIVE_FIELDS) {
    if (Object.prototype.hasOwnProperty.call(raw, key)) {
      entries.push([key, raw[key]]);
    }
  }
  return entries;
}

function serializeMaterialLine(raw) {
  const pairs = orderedEntries(raw).map(([key, value]) => `"${key}": ${serializeValue(value)}`);
  return `{${pairs.join(", ")}}`;
}

/**
 * Bouwt de volledige materialen_database.json-tekst (deterministisch: vaste
 * sleutelvolgorde, TS-arrayvolgorde, LF, UTF-8 zonder BOM, non-ASCII als \uXXXX).
 *
 * @param {Record<string, unknown>[]} rawMaterials
 * @param {{ sourceFile?: string, npmScript?: string }} [meta]
 * @returns {string}
 */
export function buildMaterialenDatabaseJson(rawMaterials, meta = {}) {
  const bronnen = ["DIN 4108-4", "NEN-EN ISO 10456", "Fabrikant specs"];
  const gegenereerdText =
    `Dit bestand is gegenereerd uit ${meta.sourceFile ?? "open-bouwlab frontend/src/lib/materialsDatabase.ts"}. ` +
    "NIET handmatig bewerken - wijzigingen gaan verloren bij de volgende generatie. " +
    `Regenereer vanuit frontend/ met: ${meta.npmScript ?? "npm run generate:materialen -- --out <pad>"}`;

  const lines = [
    "{",
    `  "versie": "3.0",`,
    `  "laatste_update": "2026-09",`,
    `  "bronnen": ${serializeValue(bronnen)},`,
    `  "_gegenereerd": ${serializeValue(gegenereerdText)},`,
    `  "materialen": [`,
  ];

  let previousCategorie = null;
  rawMaterials.forEach((raw, index) => {
    if (index > 0 && raw.categorie !== previousCategorie) {
      lines.push("");
    }
    const isLast = index === rawMaterials.length - 1;
    lines.push(`    ${serializeMaterialLine(raw)}${isLast ? "" : ","}`);
    previousCategorie = raw.categorie;
  });

  lines.push("  ]", "}");
  return `${lines.join("\n")}\n`;
}

function parseArgs(argv) {
  let out = null;
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--out" || argv[i] === "-o") {
      out = argv[i + 1];
      i += 1;
    }
  }
  return { out };
}

function main() {
  const { out } = parseArgs(process.argv.slice(2));
  const source = readFileSync(DEFAULT_SOURCE, "utf8");
  const rawMaterials = parseRawMaterials(source);
  const json = buildMaterialenDatabaseJson(rawMaterials, {
    sourceFile: "open-bouwlab frontend/src/lib/materialsDatabase.ts",
    npmScript: "npm run generate:materialen -- --out <pad>",
  });

  if (out) {
    const outPath = resolve(process.cwd(), out);
    writeFileSync(outPath, json, "utf8");
    console.log(`materialen_database.json geschreven (${rawMaterials.length} materialen) -> ${outPath}`);
  } else {
    process.stdout.write(json);
  }
}

const isMainModule =
  process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMainModule) {
  main();
}
