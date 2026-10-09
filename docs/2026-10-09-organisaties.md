# Organisaties: gebruiker -> organisatie -> huisstijl (2026-10-09)

Besluit 08-10-2026: de online opslag (Nextcloud) is uit Open Bouwlab verwijderd; multi-tenant
blijft, opgeschoond tot een bron. Een "tenant" heet in de code nu **organisatie**.

## Model

| Begrip | Bron | Gebruik |
|---|---|---|
| Lidmaatschap | Authentik-groep `org-<id>` | Welke organisatie een gebruiker heeft |
| Organisatie | `organisaties.json` (env `ORGANISATIES_CONFIG`) | Naam, huisstijlen, CORS, later abonnement |
| Huisstijl | `huisstijlen` in de organisatie | `brand` in de Reports API |

Groepen komen binnen via `X-Authentik-Groups` (browser, forward-auth) of het `groups`-veld
van Authentik `users/me` (Bearer `ak-*`).

| Groepen van de gebruiker | Gevolg |
|---|---|
| geen `org-*` | geen organisatie: rekenen en projecten werken, rapporten 403 `geen_organisatie` |
| precies een, staat in het bestand | die organisatie |
| precies een, staat niet in het bestand | fout `onbekende_organisatie` |
| meer dan een `org-*` | fout `meerdere_organisaties` |

Er is geen fallback naar een standaardorganisatie of -huisstijl.

## Configuratiebestand

Voorbeeld: `organisaties.example.json` in de repo-root.

```json
{
  "organisaties": {
    "<id>": {
      "naam": "Weergavenaam",
      "groep": "org-<id>",
      "huisstijlen": ["<brand>"],
      "standaard_huisstijl": "<brand>",
      "cors_origins": [],
      "abonnement": null
    }
  }
}
```

| Veld | Regel |
|---|---|
| `<id>` | `[a-z0-9_-]`, max 64. Is ook de tenant voor de Reports API (`X-Original-Tenant`) en de bestandsnaam van de standaard-coverfoto |
| `naam` | niet leeg; auteur/adviseur in rapporten |
| `groep` | begint met `org-`; uniek over alle organisaties |
| `huisstijlen` | niet leeg, uniek, `[a-z0-9_-]`; moeten als brand bestaan in openaec-reports |
| `standaard_huisstijl` | staat in `huisstijlen` |
| `cors_origins` | optioneel; `http(s)://`, kleine letters, geen `/` aan het eind, geen `*` |
| `abonnement` | gereserveerd, nog zonder betekenis |

Onbekende velden (typfouten) worden geweigerd. Een dubbele sleutel in de JSON wordt niet
gedetecteerd (de laatste wint); let daarop bij handmatig bewerken.

**Uitrol:** plaats het bestand op de host VOOR de container start. Ontbreekt het bij een
bind-mount, dan maakt Docker er een map van en start de API niet. Een ongeldig of onleesbaar bestand laat de API
**niet starten**. Niet gezet = geen organisaties (de API start, rapporten geven 403).

CORS: de vereniging van alle `cors_origins`, plus `http://localhost:5173` en
`http://localhost:1420` zolang `OPENAEC_ENV` niet `production` is. Er is geen tweede lijst.

## Rapporten

1. De frontend stuurt de gekozen huisstijl als `brand` (constructie: `huisstijl`), of niets.
2. De backend bepaalt de organisatie, valideert de huisstijl (leeg = standaard; buiten de lijst =
   403 `huisstijl_niet_toegestaan`) en zet `brand` altijd zelf in de body.
3. Naar de Reports API: service-token, `X-Original-Tenant: <id>`, gevalideerde `brand`.
4. Zonder `cover.image` vult de backend `REPORT_DEFAULT_COVER_DIR/<id>.jpg|png` aan.

De huisstijlkeuze verschijnt in de UI alleen bij meer dan een toegestane huisstijl.

**Afhankelijkheid openaec-reports:** die weigert een `brand` die niet gelijk is aan de tenant
(`X-Original-Tenant`). Zolang elke organisatie alleen haar eigen huisstijl heeft (`brand == id`)
werkt dat. Een tweede huisstijl (bijv. `openaec_foundation` naast `kba`) vereist eerst een
aanpassing in openaec-reports (expliciete brand van de Open Bouwlab-service toestaan).

## Een organisatie toevoegen

1. Controleer dat de huisstijl als brand bestaat in openaec-reports.
2. Maak in Authentik de groep `org-<id>` en voeg de gebruikers toe.
3. Voeg de organisatie toe aan `organisaties.json` op de server.
4. Optioneel: standaard-coverfoto `<id>.jpg` in `REPORT_DEFAULT_COVER_DIR`.
5. Herstart de container (het bestand wordt alleen bij opstart gelezen; bewerk een gemount los
   bestand in place of herstart, anders ziet de container de oude inode).
6. Controleer `GET /api/v1/me` als die gebruiker: `organisatie.id` = `<id>`.

## Projecten en opslag

Projecten staan in de eigen database, per gebruiker; ze hebben geen organisatie en zijn niet
gemigreerd. Lokaal bewaren: Backstage "Opslaan als > Lokaal bestand (.ifcenergy)" en
"Openen > Lokaal bestand"; plus "Export JSON" op Resultaten. Opslaan -> openen -> opslaan is
byte-gelijk (test in `importExport.test.ts`; alleen de IFCX-id's zijn per opslag nieuw).

## Vervallen

`/api/v1/cloud/*`, `openaec-cloud`, `tenants.json`, `TENANTS_CONFIG`, `DEFAULT_TENANT`,
`NC_SERVICE_PASS_*`, `X-Authentik-Meta-Tenant`, `attributes.tenant`, inkomende
`X-Original-Tenant` met `TENANT_OVERRIDE_ACCOUNTS`, `CORS_ORIGINS`, `OPENAEC_TENANTS_ROOT`
(`tenant.yaml`-lezer).
