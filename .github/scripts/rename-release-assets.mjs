import { chmodSync, existsSync, readdirSync, renameSync, writeFileSync } from "fs";
import { join } from "path";

const platform = process.argv[2];
const version = process.argv[3];
const slug = process.env.APP_SLUG;
const assetListPath = ".github/release-assets.txt";

if (!slug || !version || !platform) {
  console.error(
    "Usage: APP_SLUG=<slug> node .github/scripts/rename-release-assets.mjs <platform> <version>"
  );
  process.exit(1);
}

function findVsix() {
  const pluginDir = "vsc-plugin";
  if (!existsSync(pluginDir)) {
    return null;
  }

  for (const entry of readdirSync(pluginDir, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(".vsix")) {
      return join(pluginDir, entry.name);
    }
  }
  return null;
}

function renameBinary() {
  const from = join("target", "release", slug);
  if (!existsSync(from)) {
    return null;
  }

  const newName = `${slug}-${version}-linux-amd64`;
  const to = join("target", "release", newName);
  renameSync(from, to);
  chmodSync(to, 0o755);
  console.log(`Renamed: ${slug} -> ${newName}`);
  return to;
}

function renameVsix(vsixPath) {
  const newName = `${slug}-${version}.vsix`;
  const to = join("vsc-plugin", newName);
  renameSync(vsixPath, to);
  console.log(`Renamed: ${vsixPath} -> ${to}`);
  return to;
}

let files = [];

if (platform === "ubuntu-22.04") {
  const binary = renameBinary();
  if (binary) {
    files.push(binary);
  }

  const vsix = findVsix();
  if (vsix) {
    files.push(renameVsix(vsix));
  }
} else {
  console.error(`Unknown platform: ${platform}`);
  process.exit(1);
}

if (files.length === 0) {
  console.error("No release assets renamed");
  if (existsSync(join("target", "release"))) {
    console.error("target/release contents:");
    for (const entry of readdirSync(join("target", "release"), { withFileTypes: true })) {
      console.error(`  ${entry.name}${entry.isDirectory() ? "/" : ""}`);
    }
  }
  process.exit(1);
}

writeFileSync(assetListPath, `${files.join("\n")}\n`);
console.log(`Renamed ${files.length} asset(s)`);
for (const file of files) {
  console.log(`  ${file}`);
}
