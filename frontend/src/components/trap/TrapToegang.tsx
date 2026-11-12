import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "../layout/PageHeader";
import { useTrapEntitlement } from "../../hooks/useTrapEntitlement";

/**
 * Toont de trapontwerper alleen als het entitlement `trap` aan staat; anders
 * een nette melding (zelfde gedrag als `ConstructieToegang`).
 */
export function TrapToegang({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const status = useTrapEntitlement();

  if (status === "aan") return <>{children}</>;

  return (
    <div>
      <PageHeader title={t("trap.titel")} />
      <div className="p-6">
        {status === "laden" ? (
          <p role="status" className="text-sm text-on-surface-secondary">
            {t("trap.toegang.laden")}
          </p>
        ) : (
          <div
            role="alert"
            className="max-w-xl rounded-md border border-[var(--oaec-border)] bg-[var(--oaec-bg-lighter)] p-5"
          >
            <h2 className="font-heading text-base font-medium text-on-surface">
              {t("trap.toegang.nietGeactiveerdTitel")}
            </h2>
            <p className="mt-1 text-sm text-on-surface-secondary">{t("trap.toegang.nietGeactiveerd")}</p>
          </div>
        )}
      </div>
    </div>
  );
}
