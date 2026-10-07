import { ConstructiePagina } from "../components/constructie/ConstructiePagina";
import { ConstructieToegang } from "../components/constructie/ConstructieToegang";

/** Voordimensionering betonbalk (route `/constructie/beton`). */
export function ConstructieBeton() {
  return (
    <ConstructieToegang>
      <ConstructiePagina materiaal="beton" />
    </ConstructieToegang>
  );
}
