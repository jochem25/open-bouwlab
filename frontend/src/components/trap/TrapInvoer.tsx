/**
 * Invoerkolom van de trapontwerper, ingedeeld als de mockup: trapgat en
 * hoogte, verdrijving, vrije ruimte, constructie.
 */
import { useId, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { aantalArmen, type Formulier, parseGetal, VORMEN } from "./formulier";

type Getalveld = {
  [K in keyof Formulier]: Formulier[K] extends number | null ? K : never;
}[keyof Formulier];

interface Props {
  waarde: Formulier;
  onChange: (patch: Partial<Formulier>) => void;
}

function Groep({ titel, children }: { titel: string; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-3 border-t border-[var(--oaec-border)] pt-3 first:border-t-0 first:pt-0">
      <legend className="mb-1 text-xs font-semibold uppercase tracking-wide text-on-surface-secondary">{titel}</legend>
      {children}
    </fieldset>
  );
}

function Vink({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
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

export function TrapInvoer({ waarde: f, onChange }: Props) {
  const { t } = useTranslation();
  const n = aantalArmen(f.vorm);

  const getal = (veld: Getalveld, label: string, unit = "mm") => (
    <Input
      label={label}
      unit={unit}
      type="number"
      value={f[veld] ?? ""}
      onChange={(e) => onChange({ [veld]: parseGetal(e.target.value) } as Partial<Formulier>)}
    />
  );
  const arm = (i: 0 | 1 | 2) => (
    <Input
      key={i}
      label={t("trap.invoer.arm", { n: i + 1 })}
      unit="mm"
      type="number"
      value={f.armen[i] ?? ""}
      onChange={(e) => {
        const armen = [...f.armen] as Formulier["armen"];
        armen[i] = parseGetal(e.target.value);
        onChange({ armen });
      }}
    />
  );

  return (
    <div className="flex flex-col gap-4" data-testid="trap-invoer">
      <Groep titel={t("trap.invoer.groep.algemeen")}>
        <Input label={t("trap.invoer.naam")} value={f.naam} onChange={(e) => onChange({ naam: e.target.value })} />
        <div className="grid grid-cols-2 gap-2">
          <Input
            label={t("trap.invoer.projectnaam")}
            value={f.projectnaam}
            onChange={(e) => onChange({ projectnaam: e.target.value })}
          />
          <Input
            label={t("trap.invoer.bouwlaag")}
            value={f.bouwlaag}
            onChange={(e) => onChange({ bouwlaag: e.target.value })}
          />
        </div>
      </Groep>

      <Groep titel={t("trap.invoer.groep.trapgat")}>
        <Select
          label={t("trap.invoer.vorm")}
          value={f.vorm}
          options={VORMEN.map((v) => ({ value: v, label: t(`trap.vorm.${v}`) }))}
          onChange={(e) => onChange({ vorm: e.target.value as Formulier["vorm"] })}
          data-testid="trap-vorm"
        />
        <div className="grid grid-cols-2 gap-2">
          {getal("onder", t("trap.invoer.onder"))}
          {getal("verdiepingshoogte", t("trap.invoer.verdiepingshoogte"))}
        </div>
        <div className={`grid gap-2 ${n === 1 ? "grid-cols-1" : n === 2 ? "grid-cols-2" : "grid-cols-3"}`}>
          {([0, 1, 2] as const).slice(0, n).map(arm)}
        </div>
        <p className="-mt-2 text-xs text-on-surface-secondary">{t("trap.invoer.armHint")}</p>
        <div className="grid grid-cols-2 gap-2">
          {getal("breedte", t("trap.invoer.breedte"))}
          <Select
            label={t("trap.invoer.breedteIs")}
            value={f.breedte_is}
            options={[
              { value: "loopbreedte", label: t("trap.invoer.loopbreedte") },
              { value: "buitenwerks", label: t("trap.invoer.buitenwerks") },
            ]}
            onChange={(e) => onChange({ breedte_is: e.target.value as Formulier["breedte_is"] })}
          />
        </div>
        <Vink
          label={t("trap.invoer.optredenAuto")}
          checked={f.optreden_auto}
          onChange={(v) => onChange({ optreden_auto: v })}
        />
        {!f.optreden_auto && getal("optreden", t("trap.invoer.optreden"), "st")}
        <div className="grid grid-cols-2 gap-2">
          <Select
            label={t("trap.invoer.gebruiksfunctie")}
            value={f.gebruiksfunctie}
            options={[
              { value: "woonfunctie", label: t("trap.invoer.woonfunctie") },
              { value: "overig", label: t("trap.invoer.overig") },
            ]}
            onChange={(e) => onChange({ gebruiksfunctie: e.target.value as Formulier["gebruiksfunctie"] })}
          />
          <Select
            label={t("trap.invoer.bouwsoort")}
            value={f.bouwsoort}
            options={[
              { value: "nieuwbouw", label: t("trap.invoer.nieuwbouw") },
              { value: "verbouw", label: t("trap.invoer.verbouw") },
            ]}
            onChange={(e) => onChange({ bouwsoort: e.target.value as Formulier["bouwsoort"] })}
          />
        </div>
      </Groep>

      <Groep titel={t("trap.invoer.groep.verdrijving")}>
        <div className="grid grid-cols-2 gap-2">
          {getal("looplijn_afstand", t("trap.invoer.looplijnAfstand"))}
          <Select
            label={t("trap.invoer.looplijnHoek")}
            value={f.looplijn_hoek}
            options={[
              { value: "boog", label: t("trap.invoer.boog") },
              { value: "scherp", label: t("trap.invoer.scherp") },
            ]}
            onChange={(e) => onChange({ looplijn_hoek: e.target.value as Formulier["looplijn_hoek"] })}
          />
        </div>
        {n > 1 && (
          <>
            {getal("looplijn_straal", t("trap.invoer.looplijnStraal"))}
            <p className="-mt-2 text-xs text-on-surface-secondary">{t("trap.invoer.straalHint")}</p>
            <Vink
              label={t("trap.invoer.vakkenAuto")}
              checked={f.vakken_auto}
              onChange={(v) => onChange({ vakken_auto: v })}
            />
            {!f.vakken_auto && (
              <Input
                label={t("trap.invoer.vakken")}
                value={f.vakken}
                placeholder="0-5, 5-15"
                onChange={(e) => onChange({ vakken: e.target.value })}
              />
            )}
          </>
        )}
      </Groep>

      <Groep titel={t("trap.invoer.groep.vrijeRuimte")}>
        <div className="grid grid-cols-2 gap-2">
          {getal("vrije_hoogte", t("trap.invoer.vrijeHoogte"))}
          {getal("vloerdikte_boven", t("trap.invoer.vloerdikte"))}
        </div>
        <p className="-mt-2 text-xs text-on-surface-secondary">{t("trap.invoer.vrijeHoogteHint")}</p>
        <Input
          label={t("trap.invoer.trapgat")}
          value={f.trapgat}
          placeholder="x,y; x,y; x,y; x,y"
          onChange={(e) => onChange({ trapgat: e.target.value })}
        />
        <p className="-mt-2 text-xs text-on-surface-secondary">{t("trap.invoer.trapgatHint")}</p>
      </Groep>

      <Groep titel={t("trap.invoer.groep.constructie")}>
        <div className="grid grid-cols-2 gap-2">
          {getal("boom_dikte", t("trap.invoer.boomDikte"))}
          {getal("plankbreedte", t("trap.invoer.plankbreedte"))}
          {getal("voorhout", t("trap.invoer.voorhout"))}
          {getal("achterhout_min", t("trap.invoer.achterhout"))}
          {getal("nestdiepte", t("trap.invoer.nestdiepte"))}
          {getal("welstuk", t("trap.invoer.welstuk"))}
          {getal("trede_dikte", t("trap.invoer.tredeDikte"))}
          {getal("neus", t("trap.invoer.neus"))}
          <Select
            label={t("trap.invoer.stootbord")}
            value={f.stootbord}
            options={[
              { value: "dicht", label: t("trap.invoer.dicht") },
              { value: "open", label: t("trap.invoer.open") },
            ]}
            onChange={(e) => onChange({ stootbord: e.target.value as Formulier["stootbord"] })}
          />
          {getal("stootbord_dikte", t("trap.invoer.stootbordDikte"))}
          {n > 1 && getal("paal", t("trap.invoer.paal"))}
          {n > 1 && (
            <Select
              label={t("trap.invoer.doorlopend")}
              value={f.doorlopend}
              options={[
                { value: "hoger", label: t("trap.invoer.hoger") },
                { value: "lager", label: t("trap.invoer.lager") },
              ]}
              onChange={(e) => onChange({ doorlopend: e.target.value as Formulier["doorlopend"] })}
            />
          )}
        </div>
      </Groep>
    </div>
  );
}
