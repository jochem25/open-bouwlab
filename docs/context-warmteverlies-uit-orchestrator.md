# Warmteverlies — Orchestrator Context

> **Doel van dit bestand:** Geeft een Claude Code sessie in een ander project voldoende context om te integreren met de warmteverliesberekening, zonder dat project zelf te openen.

## Projectlocatie
- **Primair:** `D:\Github\warmteverliesberekening`
- **Claude project ID:** `D--Github-warmteverliesberekening`

## Doel
Warmteverliesberekening per ruimte conform **NEN-EN 12831**. Berekent transmissie- en ventilatieverliezen per ruimte en totaal voor het gebouw.

## Technologie
- Python
- Input: ruimtegegevens (handmatig of via Revit/PyRevit)
- Output: JSON per berekening

## Output datastructuur
Het project produceert JSON-bestanden in de `output/` map:

```json
{
  "project_id": "string",
  "project_naam": "string",
  "datum": "2026-01-01T00:00:00",
  "norm": "NEN-EN 12831",
  "ruimtes": [
    {
      "ruimte_id": "string",
      "naam": "string",
      "verdieping": "string",
      "oppervlakte_m2": 0.0,
      "transmissieverlies_W": 0.0,
      "ventilatieresverlies_W": 0.0,
      "totaal_W": 0.0
    }
  ],
  "totaal_gebouw_W": 0.0,
  "totaal_gebouw_kW": 0.0
}
```

## Input interface (voor PyRevit integratie)
Ruimtedata die als input wordt verwacht:
```json
{
  "ruimtes": [
    {
      "ruimte_id": "Revit Element ID of unieke string",
      "naam": "Slaapkamer 01",
      "verdieping": "1e verdieping",
      "oppervlakte_m2": 12.5,
      "geveloppervlak_m2": 8.0,
      "beglazing_m2": 2.4,
      "buitenomtrek": true
    }
  ]
}
```

## Integratiepunten
- **← PyRevit** levert ruimtegegevens uit Revit model als JSON input
- **→ Report** ontvangt JSON output voor rapportgeneratie

## Aandachtspunten
- Eenheid output altijd in Watt (W)
- `ruimte_id` moet uniek en stabiel zijn (bij voorkeur Revit Element ID)
- Norm: NEN-EN 12831:2017
