/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** When "false", DevAccounts hides + sign-out reverts to /login. Default: visible. */
  readonly VITE_SHOW_TEST_ACCOUNTS?: "true" | "false";
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
