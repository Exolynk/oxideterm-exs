#!/usr/bin/env bash
# Builds the OxideTerm ExS language plugin.
#
#   scripts/build.sh            -> dist/com.exolynk.language.exs/   (unpacked)
#                                  dist/com.exolynk.language.exs-<version>.zip
#
# Requires tree-sitter CLI 0.27.0 (`cargo install tree-sitter-cli --version 0.27.0 --locked`)
# and python3. Set TREE_SITTER=/path/to/tree-sitter to use a specific binary.
# Node.js is not needed: grammar.js is evaluated with the built-in QuickJS runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TS="${TREE_SITTER:-tree-sitter}"
GRAMMAR="$ROOT/tree-sitter-exs"

"$TS" --version | grep -q '^tree-sitter 0\.27\.0' || {
  echo "OxideTerm language plugins require tree-sitter CLI 0.27.0" >&2
  exit 1
}

ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$ROOT/plugin/plugin.json")"
OUT="$ROOT/dist/$ID"
rm -rf "$OUT"
mkdir -p "$OUT"

(cd "$GRAMMAR" && "$TS" generate --js-runtime native && "$TS" test)
(cd "$GRAMMAR" && "$TS" build --wasm -o "$OUT/parser.wasm")

cp "$GRAMMAR/queries/highlights.scm" "$OUT/highlights.scm"
cp "$ROOT/LICENSE" "$OUT/LICENSE"
cp "$ROOT/plugin/NOTICE" "$OUT/NOTICE"
cp "$ROOT/plugin/sample.json" "$OUT/sample.json"

python3 - "$ROOT/plugin/plugin.json" "$OUT" <<'PY'
import hashlib, json, os, sys, zipfile

manifest_path, out = sys.argv[1], sys.argv[2]
manifest = json.load(open(manifest_path))
digest = lambda name: hashlib.sha256(open(os.path.join(out, name), "rb").read()).hexdigest()
language = manifest["contributes"]["language"]
language["parserSha256"] = digest(manifest["runtime"]["entry"])
language["highlightsSha256"] = digest(language["highlights"])
with open(os.path.join(out, "plugin.json"), "w") as f:
    json.dump(manifest, f, indent=2)
    f.write("\n")

# The package holds plugin.json at its root. sample.json is only for verification.
files = ["plugin.json", "parser.wasm", "highlights.scm", "LICENSE", "NOTICE"]
archive = os.path.join(os.path.dirname(out), f"{manifest['id']}-{manifest['version']}.zip")
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
    for name in files:
        info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        z.writestr(info, open(os.path.join(out, name), "rb").read())
data = open(archive, "rb").read()
print(f"Built {archive}")
print(f"  size:   {len(data)} bytes")
print(f"  sha256: {hashlib.sha256(data).hexdigest()}")
PY
