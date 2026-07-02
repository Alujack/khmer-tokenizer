#!/usr/bin/env node
// Automated packaging script that builds and bundles the WebAssembly crate
// for both browsers and Node.js by inlining the WASM binary as a Base64 string.
// This guarantees zero-configuration out-of-the-box compatibility in React/Vite/Webpack
// without requiring any bundler plugins.
//
// Usage: node wasm/prepare-npm-pkg.js

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const wasmDir = __dirname;
const pkgDir = path.join(wasmDir, "pkg");
const webPkgDir = path.join(wasmDir, "pkg-web");

console.log("Building zero-config WebAssembly npm package (inlining WASM as Base64)...");

// 1. Clean previous pkg build directories
if (fs.existsSync(pkgDir)) {
  fs.rmSync(pkgDir, { recursive: true, force: true });
}
if (fs.existsSync(webPkgDir)) {
  fs.rmSync(webPkgDir, { recursive: true, force: true });
}
fs.mkdirSync(pkgDir, { recursive: true });

// 2. Build for web target using wasm-pack
console.log("Compiling WebAssembly via wasm-pack (target web)...");
execSync("wasm-pack build --release --target web --out-dir pkg-web", {
  cwd: wasmDir,
  stdio: "inherit",
});

// 3. Read the generated .wasm binary file and encode to base64
console.log("Inlining WASM binary as base64...");
const wasmFilePath = path.join(webPkgDir, "kh_tokenizer_bg.wasm");
if (!fs.existsSync(wasmFilePath)) {
  console.error("Error: WASM file not found!");
  process.exit(1);
}
const wasmBuffer = fs.readFileSync(wasmFilePath);
const wasmBase64 = wasmBuffer.toString("base64");

// 4. Read the generated JS loader and append the self-initialization script
const jsFilePath = path.join(webPkgDir, "kh_tokenizer.js");
let jsContent = fs.readFileSync(jsFilePath, "utf8");

const initScript = `
// --- Auto-initialization with inlined Base64 WASM ---
const wasmBase64 = "${wasmBase64}";
let wasmBytes;
if (typeof atob === 'function') {
  const binaryString = atob(wasmBase64);
  const len = binaryString.length;
  wasmBytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    wasmBytes[i] = binaryString.charCodeAt(i);
  }
} else if (typeof Buffer === 'function') {
  wasmBytes = Buffer.from(wasmBase64, 'base64');
}
initSync(wasmBytes);
`;

// Append the self-initialization script to the JS content
jsContent += initScript;

// 5. Write the modified JS file to the final package directory
const finalJsPath = path.join(pkgDir, "kh_tokenizer.js");
fs.writeFileSync(finalJsPath, jsContent, "utf8");

// 6. Copy types declaration file
fs.copyFileSync(path.join(webPkgDir, "kh_tokenizer.d.ts"), path.join(pkgDir, "kh_tokenizer.d.ts"));

// 7. Read and modify package.json
const rawPkgJson = fs.readFileSync(path.join(webPkgDir, "package.json"), "utf8");
const pkg = JSON.parse(rawPkgJson);

pkg.files = ["kh_tokenizer.js", "kh_tokenizer.d.ts", "LICENSE-MIT", "LICENSE-APACHE", "README.md"];
pkg.main = "kh_tokenizer.js";
pkg.module = "kh_tokenizer.js";
pkg.types = "kh_tokenizer.d.ts";
pkg.exports = {
  ".": {
    "types": "./kh_tokenizer.d.ts",
    "import": "./kh_tokenizer.js",
    "require": "./kh_tokenizer.js",
    "default": "./kh_tokenizer.js"
  }
};
pkg.sideEffects = [
  "./kh_tokenizer.js"
];

pkg.keywords = ["khmer", "nlp", "tokenizer", "segmentation", "cambodia", "wasm", "khmerTokenizer", "khmer-tokenizer", "khmer_tokenizer"];
pkg.homepage = "https://github.com/Alujack/khmer-tokenizer";
pkg.repository = {
  type: "git",
  url: "git+https://github.com/Alujack/khmer-tokenizer.git",
};

fs.writeFileSync(path.join(pkgDir, "package.json"), JSON.stringify(pkg, null, 2) + "\n");

// 8. Copy licenses and readme
fs.copyFileSync(path.join(wasmDir, "LICENSE-MIT"), path.join(pkgDir, "LICENSE-MIT"));
fs.copyFileSync(path.join(wasmDir, "LICENSE-APACHE"), path.join(pkgDir, "LICENSE-APACHE"));
fs.copyFileSync(path.join(wasmDir, "README.md"), path.join(pkgDir, "README.md"));

// 9. Cleanup temporary pkg-web dir
fs.rmSync(webPkgDir, { recursive: true, force: true });

console.log("Unified zero-config hybrid package successfully created in wasm/pkg/");
