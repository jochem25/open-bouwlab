/**
 * Afbeelding uploaden voor een rapport (voorblad/coverfoto): controle en inlezen.
 *
 * Gedeeld door de voorbladafbeelding van de warmteverliesrapportage en de
 * coverfoto van de constructiemodule; de backend controleert dezelfde grenzen.
 */
import type { CoverImage } from "../types/project";

/** Maximale bestandsgrootte (bytes). */
export const MAX_AFBEELDING_BYTES = 2 * 1024 * 1024;

export type AfbeeldingFout = "te_groot" | "type";

/** Controleer type en grootte; `null` als het bestand bruikbaar is. */
export function controleerAfbeelding(file: Pick<File, "size" | "type">): AfbeeldingFout | null {
  if (file.type !== "image/png" && file.type !== "image/jpeg") return "type";
  if (file.size > MAX_AFBEELDING_BYTES) return "te_groot";
  return null;
}

/** Lees een (gecontroleerd) bestand in als base64 zonder `data:`-prefix. */
export async function leesAfbeelding(file: File): Promise<CoverImage> {
  const dataUrl: string = await new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => resolve(String(fr.result));
    fr.onerror = () => reject(fr.error);
    fr.readAsDataURL(file);
  });
  return {
    data: dataUrl.replace(/^data:[^;]+;base64,/, ""),
    media_type: file.type as CoverImage["media_type"],
    filename: file.name,
  };
}
