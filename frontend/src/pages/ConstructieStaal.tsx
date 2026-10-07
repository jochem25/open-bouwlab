import { ConstructiePagina } from "../components/constructie/ConstructiePagina";
import { ConstructieToegang } from "../components/constructie/ConstructieToegang";

/** Voordimensionering stalen ligger (route `/constructie/staal`). */
export function ConstructieStaal() {
  return (
    <ConstructieToegang>
      <ConstructiePagina materiaal="staal" />
    </ConstructieToegang>
  );
}
