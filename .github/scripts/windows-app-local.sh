#!/usr/bin/env bash
# Gathers the DLLs the Windows app loads from its own folder and writes a Tauri config shipping
# them beside the exe (release.yml, t-1663).
#
# Usage: windows-app-local.sh EXE OUT_DIR
#   EXE is the built app (target/release/oku-slides-app.exe); DirectML.dll is read beside it,
#   where ort-sys's copy-dylibs put it. Copies the Visual C++ runtime (the VS install's own
#   redistributable folder) and DirectML.dll into OUT_DIR, then prints the path of
#   OUT_DIR/tauri.json, a `bundle.resources` map for `cargo tauri build --config`.
#   Fails when the exe imports a runtime DLL not shipped, or ONNX Runtime as a DLL (it must stay
#   static).
# Runs in Git Bash on a GitHub Windows runner: needs vswhere, cygpath and rustup's llvm-tools.
set -euo pipefail

die() { echo "::error::windows-app-local: $*" >&2; exit 1; }

[ "$#" -eq 2 ] || die "usage: windows-app-local.sh EXE OUT_DIR"
exe=$1 out=$2
[ -f "$exe" ] || die "no exe at $exe"
mkdir -p "$out"

vswhere="C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
vs=$("$vswhere" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
  -property installationPath | tr -d '\r')
[ -n "$vs" ] || die "no Visual Studio with the x64 C++ tools"
vs=$(cygpath -u "$vs")
redist=$(tr -d '\r\n ' < "$vs/VC/Auxiliary/Build/Microsoft.VCRedistVersion.default.txt")
crt=$(find "$vs/VC/Redist/MSVC/$redist/x64" -maxdepth 1 -type d -name 'Microsoft.VC14*.CRT' | head -1)
[ -n "$crt" ] || die "no x64 CRT folder in VC/Redist/MSVC/$redist"
cp "$crt"/*.dll "$out/"
# copy-dylibs leaves a symlink or a copy: -L ships the file either way.
cp -L "$(dirname "$exe")/DirectML.dll" "$out/" || die "no DirectML.dll beside $exe"

readobj="$(rustc --print sysroot)/lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-readobj.exe"
[ -x "$readobj" ] || die "no llvm-readobj (rustup component llvm-tools)"
imports=$("$readobj" --coff-imports "$exe" | sed -n 's/^ *Name: \(.*\.[dD][lL][lL]\)\r\{0,1\}$/\1/p')
echo "DLL imports of $(basename "$exe"): ${imports//$'\n'/ }" >&2
shipped=$(find "$out" -maxdepth 1 -name '*.dll' -exec basename {} \; | tr '[:upper:]' '[:lower:]')
for dll in $imports; do
  lower=$(tr '[:upper:]' '[:lower:]' <<<"$dll")
  case "$lower" in
    onnxruntime*) die "$dll is imported: ONNX Runtime must be linked statically" ;;
    vcruntime*|msvcp*|concrt*|vccorlib*|directml.dll)
      grep -qxF "$lower" <<<"$shipped" || die "$dll is imported but not shipped" ;;
  esac
done

# Absolute forward-slash paths: Tauri resolves them as is and copies each to $RESOURCE, the
# install folder on Windows, under the name given.
json=$(for f in "$out"/*.dll; do
  printf '%s\n%s\n' "$(cygpath -m "$f")" "$(basename "$f")"
done | jq -Rn '[inputs] as $l | [range(0; $l | length; 2) | {key: $l[.], value: $l[. + 1]}]
  | {bundle: {resources: from_entries}}')
printf '%s\n' "$json" > "$out/tauri.json"
echo "Shipped beside the exe: ${shipped//$'\n'/ }" >&2
cygpath -m "$out/tauri.json"
