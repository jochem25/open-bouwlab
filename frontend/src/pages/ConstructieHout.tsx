import { ConstructiePagina } from "../components/constructie/ConstructiePagina";
import { ConstructieToegang } from "../components/constructie/ConstructieToegang";

/** Voordimensionering houten balk(laag) (route `/constructie/hout`). */
export function ConstructieHout() {
  return (
    <ConstructieToegang>
      <ConstructiePagina materiaal="hout" />
    </ConstructieToegang>
  );
}
