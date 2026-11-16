#!/usr/bin/env node
/**
 * Open Heatloss Studio MCP Server
 *
 * Stelt de ISSO 51 warmteverlies-rekenkern beschikbaar via Model Context
 * Protocol — Claude Desktop, Claude Code en andere MCP-clients kunnen
 * project-bestanden openen, berekenen en inspecteren.
 *
 * Tools:
 * - calculate         : run warmteverliesberekening op een Project JSON
 * - calculate_file    : open .ifcenergy / .isso51.json en bereken
 * - parse_ifcenergy   : parseer .ifcenergy bestand naar { project, result, modeller }
 * - get_schema        : haal JSON schema (project / result) op
 * - list_constructions: standaard constructies-catalogus
 * - trap_contract      : trapontwerper headless (assemblagecontract) -> IFC,
 *                        toetsverslag Bbl, pdf
 * - trap_contract_schema: schema van de headless trapinvoer
 *
 * Resources:
 * - project://current : laatst geopende project
 * - result://current  : laatst berekende resultaat
 *
 * Pattern gespiegeld op Open Calc Studio's mcp-server/. Gebruikt het
 * `calc_from_file` cargo example als reken-engine (geen runtime Rust
 * nodig in de MCP host — wel een gebouwde binary).
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { execFile } from "node:child_process";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, "..", "..");

// ---------------------------------------------------------------------------
// In-memory state — laatst geladen project + result als resources
// ---------------------------------------------------------------------------

interface SessionState {
  projectPath: string | null;
  project: unknown | null;
  result: unknown | null;
  modeller: unknown | null;
}

const state: SessionState = {
  projectPath: null,
  project: null,
  result: null,
  modeller: null,
};

// ---------------------------------------------------------------------------
// Calc engine — shells out to the cargo example
// ---------------------------------------------------------------------------

/**
 * Resolve path naar het `calc_from_file` voorbeeld-binary.
 * Build voorwaarde: `cargo build --release -p isso51-core --example calc_from_file`
 * vanaf de repo-root.
 */
function calcBinaryPath(): string {
  // Verwacht onder target/release/examples/calc_from_file(.exe)
  const exeName = process.platform === "win32" ? "calc_from_file.exe" : "calc_from_file";
  const candidates = [
    join(REPO_ROOT, "target", "release", "examples", exeName),
    join(REPO_ROOT, "target", "debug", "examples", exeName),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  throw new Error(
    `calc_from_file binary niet gevonden. Build eerst:\n  cargo build --release -p isso51-core --example calc_from_file\n` +
      `Gezocht in:\n${candidates.map((c) => `  ${c}`).join("\n")}`,
  );
}

/**
 * Bereken Project JSON via het CLI binary. Schrijft de input naar een tijdelijk
 * bestand (binary leest envelope + raw Project beide) en parseert de stdout.
 */
async function runCalc(projectJson: unknown): Promise<unknown> {
  const tmpFile = join(REPO_ROOT, "target", `mcp-input-${Date.now()}.json`);
  writeFileSync(tmpFile, JSON.stringify({ project: projectJson }), "utf8");
  try {
    const bin = calcBinaryPath();
    const { stdout } = await execFileAsync(bin, [tmpFile], {
      maxBuffer: 64 * 1024 * 1024, // 64MB — sommige resultaten zijn groot
    });
    // Het example dumpt eerst "=== Full result JSON ===" daarna pretty JSON.
    // Pak de JSON tussen die marker en de "=== Summary ===" marker.
    const startMarker = "=== Full result JSON ===";
    const endMarker = "=== Summary ===";
    const startIdx = stdout.indexOf(startMarker);
    const endIdx = stdout.indexOf(endMarker);
    if (startIdx < 0 || endIdx < 0) {
      throw new Error(`Onverwachte stdout van calc_from_file:\n${stdout}`);
    }
    const jsonText = stdout.substring(startIdx + startMarker.length, endIdx).trim();
    return JSON.parse(jsonText);
  } finally {
    try {
      writeFileSync(tmpFile, "");
    } catch {
      // negeer
    }
  }
}

// ---------------------------------------------------------------------------
// MCP server
// ---------------------------------------------------------------------------

const server = new McpServer({
  name: "open-heatloss-studio",
  version: "0.1.0",
});

// ----- Tool: calculate ------------------------------------------------------

server.tool(
  "calculate",
  "Bereken warmteverlies (ISSO 51:2023) van een Project JSON. Geeft de full ProjectResult terug — per-room phi_HL/phi_T/phi_V plus building summary (connection_capacity, total_envelope_loss, etc).",
  {
    project: z
      .record(z.unknown())
      .describe("Volledig Project JSON object (zie schemas/v1/project.schema.json)."),
  },
  async ({ project }) => {
    const result = await runCalc(project);
    state.project = project;
    state.result = result;
    return {
      content: [
        { type: "text", text: JSON.stringify(result, null, 2) },
      ],
    };
  },
);

// ----- Tool: calculate_file -------------------------------------------------

server.tool(
  "calculate_file",
  "Open een .ifcenergy of .isso51.json bestand en bereken het project. Update ook de project://current en result://current resources.",
  {
    filePath: z.string().describe("Absoluut pad naar het bestand"),
  },
  async ({ filePath }) => {
    const content = readFileSync(filePath, "utf8");
    const parsed = JSON.parse(content);

    // Detect format: ifcenergy IFCX vs isso51-legacy envelope vs raw
    let project: unknown;
    if (parsed.header?.ifcxVersion && Array.isArray(parsed.data)) {
      // .ifcenergy: vind de envelope
      let envelope: { project: unknown; result?: unknown; modeller?: unknown } | null = null;
      for (const entry of parsed.data) {
        const env = entry.attributes?.["isso51::envelope::v1"];
        if (env) {
          envelope = env;
          break;
        }
      }
      if (!envelope) {
        throw new Error("Geen isso51::envelope::v1 in IFCX document");
      }
      project = envelope.project;
      state.modeller = envelope.modeller ?? null;
    } else if (parsed.schema === "isso51-project-v1" && parsed.project) {
      project = parsed.project;
      state.modeller = parsed.modeller ?? null;
    } else {
      project = parsed;
    }

    const result = await runCalc(project);
    state.projectPath = filePath;
    state.project = project;
    state.result = result;

    const summary = (result as { summary?: unknown })?.summary;
    return {
      content: [
        {
          type: "text",
          text:
            `Berekend uit ${filePath}\n\n` +
            `Connection capacity: ${(summary as { connection_capacity?: number })?.connection_capacity?.toFixed(1) ?? "?"} W\n` +
            `Transmissieverlies:  ${(summary as { total_envelope_loss?: number })?.total_envelope_loss?.toFixed(1) ?? "?"} W\n` +
            `Ventilatieverlies:   ${(summary as { total_ventilation_loss?: number })?.total_ventilation_loss?.toFixed(1) ?? "?"} W\n` +
            `Systeemverliezen:    ${(summary as { total_system_losses?: number })?.total_system_losses?.toFixed(1) ?? "?"} W\n\n` +
            `Volledig resultaat opvraagbaar via resource result://current.`,
        },
      ],
    };
  },
);

// ----- Tool: parse_ifcenergy ------------------------------------------------

server.tool(
  "parse_ifcenergy",
  "Parse een .ifcenergy IFCX bestand zonder te berekenen. Geeft project + result (indien aanwezig) + modeller-snapshot terug.",
  {
    filePath: z.string().describe("Absoluut pad naar het .ifcenergy bestand"),
  },
  async ({ filePath }) => {
    const content = readFileSync(filePath, "utf8");
    const parsed = JSON.parse(content);
    if (!parsed.header?.ifcxVersion || !Array.isArray(parsed.data)) {
      throw new Error("Bestand is geen geldig IFCX document");
    }
    let envelope: unknown = null;
    for (const entry of parsed.data) {
      const env = entry.attributes?.["isso51::envelope::v1"];
      if (env) {
        envelope = env;
        break;
      }
    }
    if (!envelope) {
      throw new Error("Geen isso51::envelope::v1 in document");
    }
    return {
      content: [{ type: "text", text: JSON.stringify(envelope, null, 2) }],
    };
  },
);

// ----- Tool: generate_pdf ---------------------------------------------------

/**
 * Resolve path naar het `gen_pdf` binary (built via
 * `cargo build --release --bin gen_pdf`).
 */
function genPdfBinaryPath(): string {
  const exeName = process.platform === "win32" ? "gen_pdf.exe" : "gen_pdf";
  const candidates = [
    join(REPO_ROOT, "target", "release", exeName),
    join(REPO_ROOT, "target", "debug", exeName),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  throw new Error(
    `gen_pdf binary niet gevonden. Build eerst:\n  cargo build --release --bin gen_pdf\n` +
      `(Of download het uit de CI artifact 'gen-pdf-cli'.)\n` +
      `Gezocht in:\n${candidates.map((c) => `  ${c}`).join("\n")}`,
  );
}

server.tool(
  "generate_pdf",
  "Genereer een PDF-rapport uit een project-bestand (.ifcenergy / .isso51.json / raw Project JSON). Schrijft naar het opgegeven output-pad. Vereist dat het `gen_pdf` binary is gebouwd (cargo build --release --bin gen_pdf, OF download van CI artifact gen-pdf-cli).",
  {
    inputPath: z.string().describe("Absoluut pad naar input bestand (.ifcenergy / .isso51.json / project.json)"),
    outputPath: z.string().describe("Absoluut pad waar de PDF geschreven moet worden"),
  },
  async ({ inputPath, outputPath }) => {
    const bin = genPdfBinaryPath();
    const { stdout, stderr } = await execFileAsync(bin, [inputPath, outputPath], {
      maxBuffer: 16 * 1024 * 1024,
    });
    return {
      content: [
        {
          type: "text",
          text:
            `PDF gegenereerd: ${outputPath}\n\n` +
            (stdout ? `stdout: ${stdout}\n` : "") +
            (stderr ? `stderr: ${stderr}` : ""),
        },
      ],
    };
  },
);

// ----- Tool: get_schema -----------------------------------------------------

server.tool(
  "get_schema",
  "Haal de JSON schema van project of result op (uit schemas/v1/).",
  {
    name: z.enum(["project", "result"]).describe("Welk schema"),
  },
  async ({ name }) => {
    const schemaPath = join(REPO_ROOT, "schemas", "v1", `${name}.schema.json`);
    const content = readFileSync(schemaPath, "utf8");
    return {
      content: [{ type: "text", text: content }],
    };
  },
);

// ----- Tool: list_constructions ---------------------------------------------

server.tool(
  "list_constructions",
  "Lijst van standaard constructies uit de bibliotheek (Rc-waarden, lambdas). Geeft de catalogue.json content terug.",
  {},
  async () => {
    // Catalog leeft in frontend/src/lib/constructionCatalogue als typescript;
    // voor de MCP geven we de geserialiseerde versie uit de schemas folder
    // als die bestaat, anders een placeholder.
    const candidates = [
      join(REPO_ROOT, "schemas", "v1", "construction-catalogue.json"),
      join(REPO_ROOT, "frontend", "src", "lib", "constructionCatalogue.ts"),
    ];
    for (const p of candidates) {
      if (existsSync(p)) {
        const content = readFileSync(p, "utf8");
        return {
          content: [{ type: "text", text: `Source: ${p}\n\n${content}` }],
        };
      }
    }
    return {
      content: [
        {
          type: "text",
          text:
            "Standaard constructies-catalogus niet gevonden in schemas/v1/.\n" +
            "Bekijk frontend/src/lib/constructionCatalogue.ts voor de TypeScript-bron.",
        },
      ],
    };
  },
);

// ----- Resources ------------------------------------------------------------

server.resource(
  "current-project",
  "project://current",
  {
    name: "Current project",
    description: "Het project zoals laatst ingeladen of berekend in deze sessie.",
    mimeType: "application/json",
  },
  async () => ({
    contents: [
      {
        uri: "project://current",
        mimeType: "application/json",
        text: state.project
          ? JSON.stringify(state.project, null, 2)
          : "{ \"info\": null, \"hint\": \"Gebruik calculate_file of calculate om eerst een project te laden.\" }",
      },
    ],
  }),
);

server.resource(
  "current-result",
  "result://current",
  {
    name: "Current result",
    description: "Berekend resultaat van het laatst geladen project.",
    mimeType: "application/json",
  },
  async () => ({
    contents: [
      {
        uri: "result://current",
        mimeType: "application/json",
        text: state.result
          ? JSON.stringify(state.result, null, 2)
          : "{ \"hint\": \"Gebruik calculate of calculate_file om een resultaat te genereren.\" }",
      },
    ],
  }),
);

// ---------------------------------------------------------------------------
// Trapontwerper headless (assemblagecontract kba-assemblage/1, O-2026-11-15-05)
// ---------------------------------------------------------------------------

/**
 * Pad naar de `trap` CLI (crate trap-core): `TRAP_BIN`, anders
 * target/{release,debug}/trap(.exe). Bouwen: `cargo build --release -p trap-core`.
 */
function trapBinaryPath(): string {
  const exeName = process.platform === "win32" ? "trap.exe" : "trap";
  const candidates = [
    process.env.TRAP_BIN ?? "",
    join(REPO_ROOT, "target", "release", exeName),
    join(REPO_ROOT, "target", "debug", exeName),
  ].filter((p) => p !== "");
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  throw new Error(
    `trap binary niet gevonden. Build eerst:\n  cargo build --release -p trap-core\n` +
      `of zet TRAP_BIN. Gezocht in:\n${candidates.map((c) => `  ${c}`).join("\n")}`,
  );
}

interface ToetsRegel {
  id: string;
  oordeel: string;
}

interface Toets {
  resultaat: {
    hoogte: number;
    optreden: number;
    optrede: number;
    aantreden: number;
    aantrede_looplijn: number;
    loopbreedte: number;
  };
  regels: ToetsRegel[];
  waarschuwingen: string[];
}

server.tool(
  "trap_contract",
  "Trapontwerper headless volgens het assemblagecontract: leest een invoer-JSON " +
    "(schema kba-trap-headless/1, zie trap_contract_schema) met projectverwijzing, " +
    "bouwlaag en houtsoort, en schrijft <onderdeel>.ifc (contract-IFC voor koppeling in " +
    "Revit), .toets.json, .toetsverslag.md (Bbl met artikel_status uit het normcorpus) " +
    "en .pdf (geannoteerde plattegrond). Geeft de paden en een samenvatting van de toetsing. " +
    "De CLI doet hetzelfde zonder Claude: trap contract <invoer.json> [--uit <map>].",
  {
    invoerPath: z.string().describe("Absoluut pad naar de headless invoer-JSON"),
    uitMap: z
      .string()
      .optional()
      .describe("Map voor de uitvoer; weg = naast de invoer"),
    tijd: z
      .string()
      .optional()
      .describe("Tijdstempel ISO 8601 voor FILE_NAME en verslag (reproduceerbaar); weg = nu"),
    corpus: z
      .string()
      .optional()
      .describe("Map kennis-v2 voor artikel_status; weg = %KBA_SHARED%\\kennis-v2"),
  },
  async ({ invoerPath, uitMap, tijd, corpus }) => {
    const args = ["contract", invoerPath];
    if (uitMap) args.push("--uit", uitMap);
    if (tijd) args.push("--tijd", tijd);
    if (corpus) args.push("--corpus", corpus);
    let stdout: string;
    try {
      ({ stdout } = await execFileAsync(trapBinaryPath(), args, { maxBuffer: 16 * 1024 * 1024 }));
    } catch (err) {
      // de CLI weigert met een leesbare melding op stderr (contract: weigeren i.p.v. raden)
      const e = err as { stderr?: string; message: string };
      return {
        isError: true,
        content: [{ type: "text", text: `geweigerd: ${(e.stderr ?? e.message).trim()}` }],
      };
    }
    const paden = stdout.split(/\r?\n/).filter((r) => r.trim() !== "");
    const toetsPad = paden.find((p) => p.endsWith(".toets.json"));
    let samenvatting = "";
    if (toetsPad) {
      let t: Toets;
      try {
        t = JSON.parse(readFileSync(toetsPad, "utf8")) as Toets;
      } catch (err) {
        // de bestanden zijn geschreven; alleen de samenvatting faalt
        return {
          isError: true,
          content: [
            {
              type: "text",
              text: `toetsuitvoer onleesbaar (${(err as Error).message}); bestanden:\n${paden.join("\n")}`,
            },
          ],
        };
      }
      const r = t.resultaat;
      const fout = t.regels.filter((g) => g.oordeel === "voldoet_niet").map((g) => g.id);
      const handmatig = t.regels.filter((g) => g.oordeel === "handmatig").map((g) => g.id);
      samenvatting =
        `hoogte ${r.hoogte.toFixed(0)} mm: ${r.optreden} x ${r.optrede.toFixed(1)} mm optrede, ` +
        `${r.aantreden} x ${r.aantrede_looplijn.toFixed(1)} mm aantrede op de looplijn, ` +
        `loopbreedte ${r.loopbreedte.toFixed(0)} mm\n` +
        `Bbl voldoet_niet: ${fout.length ? fout.join(", ") : "geen"}\n` +
        `handmatig te beoordelen: ${handmatig.length ? handmatig.join(", ") : "geen"}\n` +
        (t.waarschuwingen.length ? `waarschuwingen: ${t.waarschuwingen.join("; ")}\n` : "");
    }
    return {
      content: [{ type: "text", text: `${samenvatting}\nbestanden:\n${paden.map((p) => `  ${p}`).join("\n")}` }],
    };
  },
);

server.tool(
  "trap_contract_schema",
  "JSON-schema van de headless trapinvoer (kba-trap-headless/1). Het trapdeel volgt " +
    "het schema van de gewone trapinvoer zonder naam, project, niveaus en materiaal.",
  {},
  async () => {
    try {
      const { stdout } = await execFileAsync(trapBinaryPath(), ["contract-schema"], {
        maxBuffer: 4 * 1024 * 1024,
      });
      return { content: [{ type: "text", text: stdout }] };
    } catch (err) {
      const e = err as { stderr?: string; message: string };
      return { isError: true, content: [{ type: "text", text: `mislukt: ${(e.stderr ?? e.message).trim()}` }] };
    }
  },
);

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  // stdio mode: log naar stderr zodat stdout schoon blijft voor MCP
  console.error("[ohs-mcp] Open Heatloss Studio MCP server gestart (stdio)");
}

main().catch((err) => {
  console.error("[ohs-mcp] Fatale fout:", err);
  process.exit(1);
});
