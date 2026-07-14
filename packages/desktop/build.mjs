// Build Electron main/preload and bundle the Expo web export for production.
//
// Dev loop (hot reload):
//   Terminal 1: pnpm --filter @tamtri/app run web
//   Terminal 2: TAMTRI_USE_DEV_SERVER=1 pnpm --filter @tamtri/desktop run start
//
// Production bundle copies packages/app/dist → dist/renderer/app.

import { execSync } from "node:child_process";
import { cp, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { build } from "esbuild";

const jsToTs = {
  name: "js-to-ts",
  setup(b) {
    b.onResolve({ filter: /\.js$/ }, (args) => {
      if (args.kind === "entry-point" || !args.path.startsWith(".")) return undefined;
      const candidate = resolve(args.resolveDir, args.path.replace(/\.js$/, ".ts"));
      return existsSync(candidate) ? { path: candidate } : undefined;
    });
  },
};

const common = { bundle: true, sourcemap: true, logLevel: "info", plugins: [jsToTs] };
const repoRoot = resolve(import.meta.dirname, "..", "..");
const appPackage = join(repoRoot, "packages", "app");

await mkdir("dist/renderer", { recursive: true });

await build({
  ...common,
  entryPoints: ["src/main.ts"],
  outfile: "dist/main.js",
  platform: "node",
  format: "cjs",
  target: "node20",
  external: ["electron"],
});

await build({
  ...common,
  entryPoints: ["src/preload.ts"],
  outfile: "dist/preload.js",
  platform: "node",
  format: "cjs",
  target: "node20",
  external: ["electron"],
});

console.log("exporting @tamtri/app for web…");
execSync("pnpm --filter @tamtri/app run export:web", {
  cwd: repoRoot,
  stdio: "inherit",
});

const exported = join(appPackage, "dist");
const target = join(import.meta.dirname, "dist", "renderer", "app");
await cp(exported, target, { recursive: true });

// Bootstrap splash kept as a fallback when the export is missing.
await build({
  ...common,
  entryPoints: ["src/renderer/bootstrap.ts"],
  outfile: "dist/renderer/bootstrap.js",
  platform: "browser",
  format: "esm",
  target: "es2022",
});

await cp("src/renderer/index.html", "dist/renderer/index.html");

console.log("desktop bundle ready");
