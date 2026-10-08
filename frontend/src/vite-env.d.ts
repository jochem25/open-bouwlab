/// <reference types="vite/client" />

/** Build-time versienummer, geïnjecteerd via vite.config.ts define. */
declare const __APP_VERSION__: string;

interface ImportMetaEnv {
  readonly VITE_OIDC_ISSUER: string;
  readonly VITE_OIDC_CLIENT_ID: string;
  /** "1" = IFC-analyse geeft een mock-response (zie lib/ifcAnalyse.ts). */
  readonly VITE_IFC_ANALYSE_MOCK?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
