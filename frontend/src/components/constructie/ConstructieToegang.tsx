import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "../layout/PageHeader";
import { useConstructieEntitlement } from "../../hooks/useConstructieEntitlement";

/** Toont de kinderen alleen als de module geactiveerd is; anders een nette melding. */
export function ConstructieToegang({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const status = useConstructieEntitlement();

  if (status === "aan") return <>{children}</>;

  return (
    <div>
      <PageHeader title={t("constructie.titel")} />
      <div className="p-6">
        {status === "laden" ? (
          <p role="status" className="text-sm text-on-surface-secondary">
            {t("constructie.toegang.laden")}
          </p>
        ) : (
          <div
            role="alert"
            className="max-w-xl rounded-md border border-[var(--oaec-border)] bg-[var(--oaec-bg-lighter)] p-5"
          >
            <h2 className="font-heading text-base font-medium text-on-surface">
              {t("constructie.toegang.nietGeactiveerdTitel")}
            </h2>
            <p className="mt-1 text-sm text-on-surface-secondary">{t("constructie.toegang.nietGeactiveerd")}</p>
          </div>
        )}
      </div>
    </div>
  );
}
