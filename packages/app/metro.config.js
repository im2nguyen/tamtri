const { getDefaultConfig } = require("expo/metro-config");
const { resolve } = require("metro-resolver");
const fs = require("fs");
const path = require("path");

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, "../..");
const workspacePackages = [
  path.resolve(workspaceRoot, "packages/protocol/src"),
  path.resolve(workspaceRoot, "packages/client/src"),
  path.resolve(workspaceRoot, "packages/relay/src"),
];

const config = getDefaultConfig(projectRoot);
const defaultResolveRequest = config.resolver.resolveRequest ?? resolve;

config.watchFolders = [workspaceRoot];
config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, "node_modules"),
  path.resolve(workspaceRoot, "node_modules"),
];
config.resolver.disableHierarchicalLookup = true;

config.resolver.resolveRequest = (context, moduleName, platform) => {
  // Expo web resolves zustand's ESM entry (import.meta.env in middleware.mjs), which
  // throws "Cannot use 'import.meta' outside a module" in the browser bundle.
  if (moduleName === "zustand" || moduleName.startsWith("zustand/")) {
    const subpath =
      moduleName === "zustand"
        ? "index.js"
        : `${moduleName.slice("zustand/".length)}.js`;
    const zustandPath = path.resolve(workspaceRoot, "node_modules/zustand", subpath);
    if (fs.existsSync(zustandPath)) {
      return { type: "sourceFile", filePath: zustandPath };
    }
  }

  const origin = context.originModulePath;
  if (
    origin &&
    workspacePackages.some((root) => origin.startsWith(root)) &&
    moduleName.endsWith(".js")
  ) {
    const tsModuleName = moduleName.replace(/\.js$/, ".ts");
    const candidatePath = path.resolve(path.dirname(origin), tsModuleName);
    if (fs.existsSync(candidatePath)) {
      return defaultResolveRequest(context, tsModuleName, platform);
    }
  }

  return defaultResolveRequest(context, moduleName, platform);
};

module.exports = config;
