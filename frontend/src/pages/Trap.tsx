import { TrapPagina } from "../components/trap/TrapPagina";
import { TrapToegang } from "../components/trap/TrapToegang";

/** Trapontwerper (route `/trap`), achter het entitlement `trap`. */
export function Trap() {
  return (
    <TrapToegang>
      <TrapPagina />
    </TrapToegang>
  );
}
