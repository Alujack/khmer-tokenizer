#!/usr/bin/env node
// Post-processes the wasm-pack-generated package.json before `npm publish`:
// wasm-pack doesn't emit keywords/homepage, ships a repository.url npm wants
// normalized, and omits the LICENSE files from the "files" allowlist.
//
// Usage: node wasm/prepare-npm-pkg.js wasm/pkg-node

const fs = require("fs");
const path = require("path");

const dir = process.argv[2];
if (!dir) {
  console.error("usage: node prepare-npm-pkg.js <pkg-dir>");
  process.exit(1);
}

const file = path.join(dir, "package.json");
const pkg = JSON.parse(fs.readFileSync(file, "utf8"));

for (const license of ["LICENSE-MIT", "LICENSE-APACHE"]) {
  if (!pkg.files.includes(license)) pkg.files.push(license);
}
pkg.keywords = ["khmer", "nlp", "tokenizer", "segmentation", "cambodia", "wasm"];
pkg.homepage = "https://github.com/Alujack/khmer-tokenizer";
pkg.repository = {
  type: "git",
  url: "git+https://github.com/Alujack/khmer-tokenizer.git",
};

fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");
console.log(`${file}: patched (licenses, keywords, homepage, repository)`);
