// Build the Electron desktop bundles with esbuild:
//   - main process (dist/main.js, CJS, electron external)
//   - preload (dist/preload.js, CJS, electron external)
//   - bootstrap renderer (dist/renderer/bootstrap.js, ESM for the browser)
// and copy the renderer HTML. Our TS sources import with ".js" specifiers (for
// tsc's bundler resolution), so a small plugin remaps them to ".ts" on disk.

import { build } from "esbuild";
import { cp, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

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

await build({
  ...common,
  entryPoints: ["src/renderer/bootstrap.ts"],
  outfile: "dist/renderer/bootstrap.js",
  platform: "browser",
  format: "esm",
  target: "es2022",
});

await cp("src/renderer/index.html", "dist/renderer/index.html");
