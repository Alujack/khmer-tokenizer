#!/usr/bin/env node
// Automated packaging script that builds and bundles the WebAssembly crate
// for both NodeJS (CommonJS) and browser bundlers (ES Modules) like React/Vite.
//
// Usage: node wasm/prepare-npm-pkg.js

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const wasmDir = __dirname;
const pkgDir = path.join(wasmDir, "pkg");

console.log("Building hybrid npm package...");

// 1. Clean previous pkg build
if (fs.existsSync(pkgDir)) {
  fs.rmSync(pkgDir, { recursive: true, force: true });
}

// 2. Build for nodejs
console.log("Building Node.js target...");
execSync("wasm-pack build wasm --release --target nodejs --out-dir pkg/node", {
  cwd: path.join(wasmDir, ".."),
  stdio: "inherit",
});

// 3. Build for bundler (React / Vite / Webpack / ES Modules)
console.log("Building bundler target...");
execSync("wasm-pack build wasm --release --target bundler --out-dir pkg/bundler", {
  cwd: path.join(wasmDir, ".."),
  stdio: "inherit",
});

// 4. Read base package.json from bundler build
const bundlerPkgFile = path.join(pkgDir, "bundler", "package.json");
if (!fs.existsSync(bundlerPkgFile)) {
  console.error("Error: bundler package.json not found!");
  process.exit(1);
}
const pkg = JSON.parse(fs.readFileSync(bundlerPkgFile, "utf8"));

// 5. Update metadata for hybrid exports
pkg.files = ["node", "bundler", "LICENSE-MIT", "LICENSE-APACHE", "README.md"];
pkg.main = "node/kh_tokenizer.js";
pkg.module = "bundler/kh_tokenizer.js";
pkg.browser = "bundler/kh_tokenizer.js";
pkg.types = "bundler/kh_tokenizer.d.ts";
pkg.exports = {
  ".": {
    "types": "./bundler/kh_tokenizer.d.ts",
    "import": "./bundler/kh_tokenizer.js",
    "require": "./node/kh_tokenizer.js",
    "default": "./bundler/kh_tokenizer.js"
  }
};
pkg.sideEffects = [
  "./bundler/kh_tokenizer.js",
  "./node/kh_tokenizer.js"
];

pkg.keywords = ["khmer", "nlp", "tokenizer", "segmentation", "cambodia", "wasm", "khmerTokenizer", "khmer-tokenizer", "khmer_tokenizer"];
pkg.homepage = "https://github.com/Alujack/khmer-tokenizer";
pkg.repository = {
  type: "git",
  url: "git+https://github.com/Alujack/khmer-tokenizer.git",
};

// 6. Write final package.json to pkg root
fs.writeFileSync(path.join(pkgDir, "package.json"), JSON.stringify(pkg, null, 2) + "\n");

// 7. Copy licenses and readme
fs.copyFileSync(path.join(wasmDir, "LICENSE-MIT"), path.join(pkgDir, "LICENSE-MIT"));
fs.copyFileSync(path.join(wasmDir, "LICENSE-APACHE"), path.join(pkgDir, "LICENSE-APACHE"));
fs.copyFileSync(path.join(wasmDir, "README.md"), path.join(pkgDir, "README.md"));

// 8. Clean up redundant package.json and license files in subdirectories
const cleanList = [
  path.join(pkgDir, "node", "package.json"),
  path.join(pkgDir, "node", "LICENSE-MIT"),
  path.join(pkgDir, "node", "LICENSE-APACHE"),
  path.join(pkgDir, "node", "README.md"),
  path.join(pkgDir, "node", ".gitignore"),
  path.join(pkgDir, "bundler", "package.json"),
  path.join(pkgDir, "bundler", "LICENSE-MIT"),
  path.join(pkgDir, "bundler", "LICENSE-APACHE"),
  path.join(pkgDir, "bundler", "README.md"),
  path.join(pkgDir, "bundler", ".gitignore"),
];

for (const file of cleanList) {
  if (fs.existsSync(file)) {
    fs.unlinkSync(file);
  }
}

console.log("Hybrid package successfully created in wasm/pkg/");
