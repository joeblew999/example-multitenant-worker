import { Miniflare } from "miniflare";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const buildDir = resolve(__dirname, "..", "..", "build");
const migrationsDir = resolve(__dirname, "..", "..", "migrations");

const jsCode = readFileSync(resolve(buildDir, "index.js"), "utf-8");
const wasmBytes = readFileSync(resolve(buildDir, "index_bg.wasm"));

// Deterministic 32-byte test key, base64-encoded. Used as SESSION_KEY so
// the macaroon root is stable across worker reloads inside a test run.
const TEST_SESSION_KEY = Buffer.alloc(32, "k").toString("base64");

export const mf = new Miniflare({
  workers: [
    {
      name: "workers-multitenant",
      modules: [
        { type: "ESModule", path: "index.js", contents: jsCode },
        {
          type: "CompiledWasm",
          path: "index_bg.wasm",
          contents: wasmBytes,
        },
      ],
      compatibilityDate: "2026-04-22",
      d1Databases: ["DB"],
      bindings: {
        SESSION_KEY: TEST_SESSION_KEY,
        SESSION_TTL_SECONDS: "86400",
        INVITATION_TTL_SECONDS: "604800",
        PASSWORD_RESET_TTL_SECONDS: "900",
        EMAIL_VERIFY_TTL_SECONDS: "86400",
        SSO_STATE_TTL_SECONDS: "600",
        ENFORCE_EMAIL_VERIFICATION: "",
      },
    },
  ],
});

export const mfUrl = (await mf.ready).toString().replace(/\/$/, "");

async function applyMigrations() {
  const db = await mf.getD1Database("DB");
  const sql = readFileSync(resolve(migrationsDir, "0001_init.sql"), "utf-8");
  const cleanSql = sql
    .split("\n")
    .map((line) => line.replace(/--.*$/, ""))
    .join("\n");
  const statements = cleanSql
    .split(";")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  await db.batch(statements.map((s) => db.prepare(s)));
}

await applyMigrations();
