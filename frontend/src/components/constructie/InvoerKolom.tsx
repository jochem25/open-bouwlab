import { useId } from "react";
import { useTranslation } from "react-i18next";

import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import type {
  Gebouwtype,
  Gebruiksfunctie,
  Materiaal,
  Milieuklasse,
  Reeks,
  Scheidingswanden,
  Staalsoort,
  Sterkteklasse,
  Toepassing,
} from "../../types/constructie";
import { type Formulier, parseGetal, toegestaneGevolgklassen } from "./formulier";

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
const REEKSEN: Reeks[] = ["IPE", "HEA", "HEB"];
const STERKTEKLASSEN: Sterkteklasse[] = ["C20/25", "C25/30", "C30/37", "C35/45"];
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
        <Input
          id={`${idPrefix}-belastingbreedte`}
          type="number"
          label={t("constructie.invoer.belastingbreedte")}
          unit="m"
          value={f.belastingbreedte_m ?? ""}
          onChange={num("belastingbreedte_m")}
        />
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

      {materiaal === "staal" ? (
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
