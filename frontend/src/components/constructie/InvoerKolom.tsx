import { useId } from "react";
import { useTranslation } from "react-i18next";

import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import type {
  Gebouwtype,
  Gebruiksfunctie,
  Houtklasse,
  Materiaal,
  Milieuklasse,
  Reeks,
  Scheidingswanden,
  Staalsoort,
  Sterkteklasse,
  Toepassing,
  VloerlaagSoort,
} from "../../types/constructie";
import { Button } from "../ui/Button";
import {
  type Formulier,
  heeftTrillingSchakelaar,
  nieuweLaag,
  parseGetal,
  toegestaneGevolgklassen,
  type VloerlaagRij,
} from "./formulier";

const TOEPASSINGEN: Toepassing[] = ["vloer", "dak"];
const GEBRUIKSFUNCTIES: Gebruiksfunctie[] = [
  "woon_vloer",
  "woon_trap",
  "woon_balkon",
  "gemeenschappelijk",
  "kantoor",
];
const GEBOUWTYPES: Gebouwtype[] = [
  "eengezinswoning1_tot3",
  "eengezinswoning4_plus",
  "woongebouw",
  "kantoorgebouw",
];
const WANDEN: Scheidingswanden[] = ["geen", "tot_een", "tot_twee", "tot_drie"];
const STAALSOORTEN: Staalsoort[] = ["S235", "S275", "S355"];
const REEKSEN: Reeks[] = ["IPE", "HEA", "HEB", "UNP"];
const STERKTEKLASSEN: Sterkteklasse[] = ["C20/25", "C25/30", "C30/37", "C35/45"];
const HOUTKLASSEN: Houtklasse[] = ["C14", "C16", "C18", "C20", "C22", "C24", "C27", "C30"];
const LAAGSOORTEN: VloerlaagSoort[] = ["vloerplaat", "dekvloer", "plafond", "overig"];
const HOH_WAARDEN = [300, 400, 500, 600];
const HOUT_BREEDTES = [46, 58, 71, 96];
const MILIEUKLASSEN: Milieuklasse[] = ["XC1", "XC3"];

function Vink({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  const id = useId();
  return (
    <div className="flex items-center gap-2">
      <input
        id={id}
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="h-4 w-4 accent-[var(--theme-accent)]"
      />
      <label htmlFor={id} className="text-sm text-on-surface">
        {label}
      </label>
    </div>
  );
}

interface Props {
  materiaal: Materiaal;
  waarde: Formulier;
  onChange: (patch: Partial<Formulier>) => void;
}

/** Invoerkolom; dak-velden alleen bij dak, hoogte alleen bij vaste hoogte. */
export function InvoerKolom({ materiaal, waarde: f, onChange }: Props) {
  const { t } = useTranslation();
  const idPrefix = `constructie-${materiaal}`;
  const num = (veld: keyof Formulier) => (e: { target: { value: string } }) =>
    onChange({ [veld]: parseGetal(e.target.value) } as Partial<Formulier>);

  const opties = <T extends string>(groep: string, waarden: T[]) =>
    waarden.map((v) => ({ value: v, label: t(`constructie.opties.${groep}.${v}`) }));

  const gevolgklassen = toegestaneGevolgklassen(f.gebouwtype);

  return (
    <form
      className="flex flex-col gap-4"
      aria-label={t("constructie.invoer.titel")}
      onSubmit={(e) => e.preventDefault()}
    >
      <fieldset className="flex flex-col gap-3">
        <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
          {t("constructie.invoer.groepen.veld")}
        </legend>
        <Select
          id={`${idPrefix}-toepassing`}
          label={t("constructie.invoer.toepassing")}
          value={f.toepassing}
          options={opties("toepassing", TOEPASSINGEN)}
          onChange={(e) => onChange({ toepassing: e.target.value as Toepassing })}
        />
        <Select
          id={`${idPrefix}-gebruiksfunctie`}
          label={t("constructie.invoer.gebruiksfunctie")}
          value={f.gebruiksfunctie}
          options={opties("gebruiksfunctie", GEBRUIKSFUNCTIES)}
          onChange={(e) => onChange({ gebruiksfunctie: e.target.value as Gebruiksfunctie })}
        />
        <Select
          id={`${idPrefix}-gebouwtype`}
          label={t("constructie.invoer.gebouwtype")}
          value={f.gebouwtype}
          options={opties("gebouwtype", GEBOUWTYPES)}
          onChange={(e) => {
            const gebouwtype = e.target.value as Gebouwtype;
            const toegestaan = toegestaneGevolgklassen(gebouwtype);
            onChange({
              gebouwtype,
              gevolgklasse:
                f.gevolgklasse !== "auto" && !toegestaan.includes(f.gevolgklasse)
                  ? "auto"
                  : f.gevolgklasse,
            });
          }}
        />
        <Select
          id={`${idPrefix}-gevolgklasse`}
          label={t("constructie.invoer.gevolgklasse")}
          value={f.gevolgklasse}
          options={[
            { value: "auto", label: t("constructie.opties.gevolgklasse.auto") },
            ...gevolgklassen.map((k) => ({ value: k, label: k })),
          ]}
          onChange={(e) => onChange({ gevolgklasse: e.target.value as Formulier["gevolgklasse"] })}
        />
        {f.toepassing === "dak" && (
          <>
            <Vink
              label={t("constructie.invoer.dakBeloopbaar")}
              checked={f.dak_beloopbaar}
              onChange={(v) => onChange({ dak_beloopbaar: v })}
            />
            <Input
              id={`${idPrefix}-hellingshoek`}
              type="number"
              label={t("constructie.invoer.dakHellingshoek")}
              unit="°"
              value={f.dak_hellingshoek_graden ?? ""}
              onChange={num("dak_hellingshoek_graden")}
            />
            <Input
              id={`${idPrefix}-afschot`}
              type="number"
              label={t("constructie.invoer.dakAfschot")}
              unit="%"
              value={f.dak_afschot_procent ?? ""}
              onChange={num("dak_afschot_procent")}
            />
          </>
        )}
      </fieldset>

      <fieldset className="flex flex-col gap-3">
        <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
          {t("constructie.invoer.groepen.belasting")}
        </legend>
        <Input
          id={`${idPrefix}-overspanning`}
          type="number"
          label={t("constructie.invoer.overspanning")}
          unit="m"
          value={f.overspanning_m ?? ""}
          onChange={num("overspanning_m")}
        />
        {!(materiaal === "hout" && f.hout_element === "balklaag") && (
          <Input
            id={`${idPrefix}-belastingbreedte`}
            type="number"
            label={t("constructie.invoer.belastingbreedte")}
            unit="m"
            value={f.belastingbreedte_m ?? ""}
            onChange={num("belastingbreedte_m")}
          />
        )}
        <Input
          id={`${idPrefix}-permanent`}
          type="number"
          label={t("constructie.invoer.permanent")}
          unit="kN/m²"
          value={f.permanent_kn_m2 ?? ""}
          onChange={num("permanent_kn_m2")}
        />
        <Vink
          label={t("constructie.invoer.eigenGewichtAutomatisch")}
          checked={f.eigen_gewicht_automatisch}
          onChange={(v) => onChange({ eigen_gewicht_automatisch: v })}
        />
        <Select
          id={`${idPrefix}-wanden`}
          label={t("constructie.invoer.lichteWanden")}
          value={f.lichte_scheidingswanden}
          options={opties("wanden", WANDEN)}
          onChange={(e) => onChange({ lichte_scheidingswanden: e.target.value as Scheidingswanden })}
        />
        <Vink
          label={t("constructie.invoer.scheurgevoeligeWanden")}
          checked={f.scheurgevoelige_scheidingswanden}
          onChange={(v) => onChange({ scheurgevoelige_scheidingswanden: v })}
        />
        <Vink
          label={t("constructie.invoer.uiterlijk")}
          checked={f.uiterlijk_van_belang}
          onChange={(v) => onChange({ uiterlijk_van_belang: v })}
        />
      </fieldset>

      {materiaal === "hout" ? (
        <>
          <fieldset className="flex flex-col gap-3">
            <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
              {t("constructie.invoer.groepen.hout")}
            </legend>
            <Select
              id={`${idPrefix}-element`}
              label={t("constructie.invoer.houtElement")}
              value={f.hout_element}
              options={opties("houtElement", ["balklaag", "balk"] as const)}
              onChange={(e) => onChange({ hout_element: e.target.value as Formulier["hout_element"] })}
            />
            {f.hout_element === "balklaag" && (
              <Select
                id={`${idPrefix}-hoh`}
                label={t("constructie.invoer.hoh")}
                value={String(f.hoh_mm ?? 400)}
                options={HOH_WAARDEN.map((h) => ({ value: String(h), label: `${h} mm` }))}
                onChange={(e) => onChange({ hoh_mm: Number(e.target.value) })}
              />
            )}
            <Select
              id={`${idPrefix}-houtklasse`}
              label={t("constructie.invoer.sterkteklasse")}
              value={f.houtklasse}
              options={HOUTKLASSEN.map((s) => ({ value: s, label: s }))}
              onChange={(e) => onChange({ houtklasse: e.target.value as Houtklasse })}
            />
            <Select
              id={`${idPrefix}-houtbreedte`}
              label={t("constructie.invoer.houtBreedte")}
              value={String(f.hout_breedte_mm ?? 71)}
              options={HOUT_BREEDTES.map((b) => ({ value: String(b), label: `${b} mm` }))}
              onChange={(e) => onChange({ hout_breedte_mm: Number(e.target.value) })}
            />
            <Vink
              label={t("constructie.invoer.hoogteAutomatisch")}
              checked={f.hoogte_automatisch}
              onChange={(v) => onChange({ hoogte_automatisch: v })}
            />
            {!f.hoogte_automatisch && (
              <Input
                id={`${idPrefix}-hoogte`}
                type="number"
                label={t("constructie.invoer.hoogte")}
                unit="mm"
                value={f.hoogte_mm ?? ""}
                onChange={num("hoogte_mm")}
              />
            )}
            <Select
              id={`${idPrefix}-klimaatklasse`}
              label={t("constructie.invoer.klimaatklasse")}
              value={String(f.klimaatklasse)}
              options={[
                { value: "1", label: "1" },
                { value: "2", label: "2" },
              ]}
              onChange={(e) => onChange({ klimaatklasse: e.target.value === "2" ? 2 : 1 })}
            />
            <Vink
              label={t("constructie.invoer.drukrandGesteund")}
              checked={f.drukrand_gesteund}
              onChange={(v) => onChange({ drukrand_gesteund: v })}
            />
          </fieldset>
          {f.hout_element === "balklaag" && (
            <fieldset className="flex flex-col gap-3" data-testid="vloerplaat-blok">
              <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
                {t("constructie.invoer.groepen.vloerplaat")}
              </legend>
              <ul className="flex flex-col gap-3" data-testid="vloerlagen-lijst">
                {f.vloerlagen.map((rij, n) => {
                  const wijzigRij = (patch: Partial<VloerlaagRij>) =>
                    onChange({
                      vloerlagen: f.vloerlagen.map((r) => (r.sleutel === rij.sleutel ? { ...r, ...patch } : r)),
                    });
                  return (
                    <li key={rij.sleutel} className="flex flex-col gap-2 rounded-md border border-[var(--oaec-border-subtle)] p-2">
                      <Select
                        id={`${idPrefix}-laag-${n}-soort`}
                        label={t("constructie.invoer.laagSoort")}
                        value={rij.soort}
                        options={LAAGSOORTEN.map((s) => ({
                          value: s,
                          label: t(`constructie.invoer.laagSoorten.${s}`),
                        }))}
                        onChange={(e) => wijzigRij({ soort: e.target.value as VloerlaagSoort })}
                      />
                      <Input
                        id={`${idPrefix}-laag-${n}-dikte`}
                        type="number"
                        label={t("constructie.invoer.laagDikte")}
                        unit="mm"
                        placeholder="mm"
                        value={rij.dikte_mm ?? ""}
                        onChange={(e) => wijzigRij({ dikte_mm: parseGetal(e.target.value) })}
                      />
                      <Input
                        id={`${idPrefix}-laag-${n}-e`}
                        type="number"
                        label={t("constructie.invoer.laagE")}
                        unit="N/mm²"
                        placeholder="N/mm²"
                        value={rij.e_n_mm2 ?? ""}
                        onChange={(e) => wijzigRij({ e_n_mm2: parseGetal(e.target.value) })}
                      />
                      <Button
                        type="button"
                        variant="secondary"
                        size="sm"
                        onClick={() => onChange({ vloerlagen: f.vloerlagen.filter((r) => r.sleutel !== rij.sleutel) })}
                      >
                        {t("constructie.invoer.laagVerwijder")}
                      </Button>
                    </li>
                  );
                })}
              </ul>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={() => onChange({ vloerlagen: [...f.vloerlagen, nieuweLaag()] })}
              >
                {t("constructie.invoer.laagToevoegen")}
              </Button>
              <p className="text-xs text-on-surface-secondary">{t("constructie.invoer.laagHelp")}</p>
              <Input
                id={`${idPrefix}-vloerbreedte`}
                type="number"
                label={t("constructie.invoer.vloerbreedte")}
                unit="m"
                value={f.vloerbreedte_m ?? ""}
                onChange={num("vloerbreedte_m")}
              />
            </fieldset>
          )}
          {f.hout_element === "balklaag" && (
            <fieldset className="flex flex-col gap-3" data-testid="dwarsverbinding-blok">
              <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
                {t("constructie.invoer.groepen.dwarsverbinding")}
              </legend>
              <Input
                id={`${idPrefix}-dwars-ei`}
                type="number"
                label={t("constructie.invoer.dwarsEi")}
                unit="kNm²"
                placeholder="kNm²"
                value={f.dwars_ei_knm2 ?? ""}
                onChange={num("dwars_ei_knm2")}
              />
              <p className="text-xs text-on-surface-secondary">{t("constructie.invoer.dwarsHelp")}</p>
              {heeftTrillingSchakelaar(f, materiaal) && (
                <Vink
                  label={t("constructie.invoer.trillingstoets")}
                  checked={f.trillingstoets}
                  onChange={(v) => onChange({ trillingstoets: v })}
                />
              )}
            </fieldset>
          )}
        </>
      ) : materiaal === "staal" ? (
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
            {t("constructie.invoer.groepen.staal")}
          </legend>
          <Select
            id={`${idPrefix}-staalsoort`}
            label={t("constructie.invoer.staalsoort")}
            value={f.staalsoort}
            options={STAALSOORTEN.map((s) => ({ value: s, label: s }))}
            onChange={(e) => onChange({ staalsoort: e.target.value as Staalsoort })}
          />
          <div role="group" aria-label={t("constructie.invoer.reeksen")} className="flex flex-col gap-1">
            <span className="text-xs font-medium text-on-surface-secondary">
              {t("constructie.invoer.reeksen")}
            </span>
            <div className="flex gap-4">
              {REEKSEN.map((r) => (
                <Vink
                  key={r}
                  label={r}
                  checked={f.reeksen.includes(r)}
                  onChange={(aan) =>
                    onChange({
                      reeksen: aan
                        ? REEKSEN.filter((x) => x === r || f.reeksen.includes(x))
                        : f.reeksen.filter((x) => x !== r),
                    })
                  }
                />
              ))}
            </div>
            {f.reeksen.length === 0 && (
              <p role="alert" className="text-xs text-red-400">
                {t("constructie.invoer.reeksenLeeg")}
              </p>
            )}
          </div>
          <Vink
            label={t("constructie.invoer.bovenflensGesteund")}
            checked={f.bovenflens_gesteund}
            onChange={(v) => onChange({ bovenflens_gesteund: v })}
          />
        </fieldset>
      ) : (
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 font-heading text-sm font-medium text-on-surface">
            {t("constructie.invoer.groepen.beton")}
          </legend>
          <Select
            id={`${idPrefix}-sterkteklasse`}
            label={t("constructie.invoer.sterkteklasse")}
            value={f.sterkteklasse}
            options={STERKTEKLASSEN.map((s) => ({ value: s, label: s }))}
            onChange={(e) => onChange({ sterkteklasse: e.target.value as Sterkteklasse })}
          />
          <Input
            id={`${idPrefix}-balkbreedte`}
            type="number"
            label={t("constructie.invoer.balkbreedte")}
            unit="mm"
            value={f.balkbreedte_mm ?? ""}
            onChange={num("balkbreedte_mm")}
          />
          <Vink
            label={t("constructie.invoer.hoogteAutomatisch")}
            checked={f.hoogte_automatisch}
            onChange={(v) => onChange({ hoogte_automatisch: v })}
          />
          {!f.hoogte_automatisch && (
            <Input
              id={`${idPrefix}-hoogte`}
              type="number"
              label={t("constructie.invoer.hoogte")}
              unit="mm"
              value={f.hoogte_mm ?? ""}
              onChange={num("hoogte_mm")}
            />
          )}
          <Select
            id={`${idPrefix}-milieuklasse`}
            label={t("constructie.invoer.milieuklasse")}
            value={f.milieuklasse}
            options={MILIEUKLASSEN.map((s) => ({ value: s, label: s }))}
            onChange={(e) => onChange({ milieuklasse: e.target.value as Milieuklasse })}
          />
          <Input
            id={`${idPrefix}-phi-hoofd`}
            type="number"
            label={t("constructie.invoer.phiHoofd")}
            unit="mm"
            value={f.phi_hoofd_mm ?? ""}
            onChange={num("phi_hoofd_mm")}
          />
          <Input
            id={`${idPrefix}-phi-beugel`}
            type="number"
            label={t("constructie.invoer.phiBeugel")}
            unit="mm"
            value={f.phi_beugel_mm ?? ""}
            onChange={num("phi_beugel_mm")}
          />
          <Input
            id={`${idPrefix}-dg`}
            type="number"
            label={t("constructie.invoer.dg")}
            unit="mm"
            value={f.d_g_mm ?? ""}
            onChange={num("d_g_mm")}
          />
        </fieldset>
      )}
    </form>
  );
}
