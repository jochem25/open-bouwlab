// Rooktest trapontwerper (O-2026-10-30-14): ontwerpt drie trappen (recht, kwart,
// half) via het scherm, controleert plattegrond, toetsing, 3D en beide IFC-downloads
// en maakt screenshots. Headless Chrome via playwright-core (channel "chrome"), zoals
// de viewer-spike; playwright-core zit niet in de repo-dependencies.
//
// Gebruik (app draait, bv. scripts/trap-lokaal.ps1 of vite dev + API):
//   PLAYWRIGHT_CORE=<pad>/node_modules/playwright-core \
//   node frontend/scripts/trap-rooktest.mjs [http://localhost:3011] [uitmap]
// Exit 0 = alle controles geslaagd.
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_CORE ?? "playwright-core");

const BASIS = process.argv[2] ?? "http://localhost:3011";
const UIT = process.argv[3] ?? "target/trap-rooktest";
const WACHT_MS = 60_000;
const BREEDTE = 1600;
const HOOGTE = 1300;

const TRAPPEN = [
  { naam: "recht", vorm: "recht", armen: ["3300"] },
  { naam: "kwart", vorm: "kwart_links", armen: ["2400", "1600"] },
  { naam: "half", vorm: "half_rechts", armen: ["1500", "2200", "1500"] },
];

const fouten = [];
function controleer(ok, tekst) {
  console.log(`${ok ? "ok  " : "FOUT"} ${tekst}`);
  if (!ok) fouten.push(tekst);
}

fs.mkdirSync(UIT, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--use-angle=swiftshader"] });
const page = await browser.newPage({ viewport: { width: BREEDTE, height: HOOGTE }, acceptDownloads: true });
page.on("pageerror", (e) => console.log(`pagina-fout: ${e.message}`));
await page.goto(`${BASIS}/trap`);
await page.getByTestId("trap-invoer").waitFor({ timeout: WACHT_MS });
await page.getByTestId("trap-samenvatting").waitFor({ timeout: WACHT_MS });
controleer((await page.getByTestId("trap-disclaimer").innerText()).includes("Opgesteld door Kolthof Bouwadvies."), "disclaimer zichtbaar");

for (const trap of TRAPPEN) {
  const vorige = await page.getByTestId("trap-samenvatting").innerText().catch(() => "");
  await page.getByTestId("trap-vorm").selectOption(trap.vorm);
  for (let i = 0; i < trap.armen.length; i++) {
    const veld = page.getByLabel(`Arm ${i + 1}`, { exact: true });
    await veld.fill(trap.armen[i]);
    await veld.press("Tab");
  }
  await page.getByLabel("Naam trap").fill(`rooktest ${trap.naam}`);
  await page.getByLabel("Naam trap").press("Tab");
  await page.getByTestId("trap-tab-plattegrond").click().catch(() => {});
  await page.waitForFunction(
    (v) => {
      const s = document.querySelector('[data-testid="trap-samenvatting"]');
      return s && s.textContent !== v && !document.querySelector('[aria-busy="true"]');
    },
    vorige,
    { timeout: WACHT_MS },
  );
  await page.waitForFunction(() => !document.querySelector('[aria-busy="true"]'), null, { timeout: WACHT_MS });
  const fout = await page.getByTestId("trap-fout").count();
  controleer(fout === 0, `${trap.naam}: geen rekenfout`);
  const samenvatting = await page.getByTestId("trap-samenvatting").innerText();
  console.log(`     ${samenvatting}`);
  const regels = await page.locator('[data-testid="trap-toetsing"] tr[data-oordeel]').count();
  controleer(regels >= 15, `${trap.naam}: toetsing met ${regels} regels`);
  controleer((await page.getByTestId("trap-klimlijn").count()) === 1, `${trap.naam}: klimlijn getekend`);
  await page.screenshot({ path: path.join(UIT, `${trap.naam}_scherm.png`), fullPage: true });
  await page.getByTestId("trap-plattegrond").screenshot({ path: path.join(UIT, `${trap.naam}_plattegrond.png`) });

  // 3D: wacht tot web-ifc het model heeft geladen
  await page.getByTestId("trap-tab-3d").click();
  await page.locator('[data-testid="trap-3d"][data-status="klaar"], [data-testid="trap-3d"][data-status="fout"]').waitFor({ timeout: WACHT_MS });
  controleer((await page.getByTestId("trap-3d").getAttribute("data-status")) === "klaar", `${trap.naam}: 3D geladen`);
  await page.waitForTimeout(800);
  await page.getByTestId("trap-3d").screenshot({ path: path.join(UIT, `${trap.naam}_3d_clearance.png`) });
  await page.getByTestId("trap-clearance").uncheck();
  await page.waitForTimeout(500);
  await page.getByTestId("trap-3d").screenshot({ path: path.join(UIT, `${trap.naam}_3d.png`) });
  await page.getByTestId("trap-clearance").check();

  // downloads
  for (const [knop, proxy] of [["trap-ifc", true], ["trap-ifc-leverancier", false]]) {
    const [dl] = await Promise.all([page.waitForEvent("download"), page.getByTestId(knop).click()]);
    const doel = path.join(UIT, dl.suggestedFilename());
    await dl.saveAs(doel);
    const tekst = fs.readFileSync(doel, "utf8");
    controleer(
      tekst.includes("IFCSTAIR(") && tekst.includes("IFCBUILDINGELEMENTPROXY(") === proxy,
      `${trap.naam}: ${dl.suggestedFilename()} (${proxy ? "met" : "zonder"} clearance-proxy)`,
    );
  }
  await page.getByTestId("trap-tab-plattegrond").click();
}

// invoer opslaan en weer laden
const [dl] = await Promise.all([page.waitForEvent("download"), page.getByTestId("trap-opslaan").click()]);
const json = path.join(UIT, dl.suggestedFilename());
await dl.saveAs(json);
await page.getByTestId("trap-vorm").selectOption("recht");
await page.getByTestId("trap-laden-bestand").setInputFiles(json);
await page.waitForTimeout(300);
controleer((await page.getByTestId("trap-vorm").inputValue()) === "half_rechts", "invoer opslaan en laden");

// kwartdraai met onderkwart als vrije contour (golden 3, O-2026-11-12-01): via 'invoer laden'
const CONTOUR = "crates/trap-core/tests/fixtures/invoer_kwartdraai_onderkwart.json";
if (fs.existsSync(CONTOUR)) {
  const vorige = await page.getByTestId("trap-samenvatting").innerText().catch(() => "");
  await page.getByTestId("trap-laden-bestand").setInputFiles(CONTOUR);
  await page.waitForFunction(
    (v) => {
      const s = document.querySelector('[data-testid="trap-samenvatting"]');
      return s && s.textContent !== v && !document.querySelector('[aria-busy="true"]');
    },
    vorige,
    { timeout: WACHT_MS },
  );
  controleer((await page.getByTestId("trap-vorm").inputValue()) === "contour", "onderkwart: contour geladen");
  controleer((await page.getByTestId("trap-fout").count()) === 0, "onderkwart: geen rekenfout");
  const samenvatting = await page.getByTestId("trap-samenvatting").innerText();
  console.log(`     ${samenvatting}`);
  controleer(samenvatting.includes("16 optreden"), "onderkwart: 16 optreden");
  await page.getByTestId("trap-tab-plattegrond").click().catch(() => {});
  await page.screenshot({ path: path.join(UIT, "onderkwart_scherm.png"), fullPage: true });
  await page.getByTestId("trap-plattegrond").screenshot({ path: path.join(UIT, "onderkwart_plattegrond.png") });
  await page.getByTestId("trap-tab-3d").click();
  await page.locator('[data-testid="trap-3d"][data-status="klaar"], [data-testid="trap-3d"][data-status="fout"]').waitFor({ timeout: WACHT_MS });
  controleer((await page.getByTestId("trap-3d").getAttribute("data-status")) === "klaar", "onderkwart: 3D geladen");
  await page.waitForTimeout(800);
  await page.getByTestId("trap-3d").screenshot({ path: path.join(UIT, "onderkwart_3d_clearance.png") });
} else {
  console.log(`     onderkwart overgeslagen: ${CONTOUR} niet gevonden (draai vanuit de repo-root)`);
}

await browser.close();
console.log(fouten.length === 0 ? "rooktest geslaagd" : `rooktest: ${fouten.length} fout(en)`);
process.exit(fouten.length === 0 ? 0 : 1);
