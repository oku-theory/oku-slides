#!/usr/bin/env bash
# Tests release-sign.sh and release-gate.sh on a fake release (t-977, t-1210): `just release-sign-test`.
# Needs minisign, jq and cargo-tauri (`cargo tauri signer generate`) on PATH. Builds a draft-like
# latest.json shaped like v8.0.0's, signed with a throwaway key as tauri-action leaves it, re-signs
# it with a "real" key, then checks the gate accepts that and refuses forged or stale variants.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
sign="$here/release-sign.sh"
gate="$here/release-gate.sh"
repo=oku-theory/oku-slides
v=9.1.0
prefix="https://github.com/$repo/releases/download/v$v/"

for tool in minisign jq cargo; do
  command -v "$tool" > /dev/null || { echo "release-sign-test: $tool not found" >&2; exit 2; }
done

tmp=$(mktemp -d "${TMPDIR:-/tmp}/release-sign-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
failures=0
pass() { echo "PASS $1"; }
fail() { echo "FAIL $1"; failures=$((failures + 1)); }

# Keys: the throwaway one of the build jobs (empty password) and the real one (with a password).
cargo tauri signer generate --ci -p "" -w "$tmp/throwaway.key" -f > /dev/null 2>&1
cargo tauri signer generate --ci -p "real pw" -w "$tmp/real.key" -f > /dev/null 2>&1
real_pub=$(tr -d '\r\n' < "$tmp/real.key.pub")

# Fake bundles under their asset names.
mkdir -p "$tmp/bundles"
names="Oku.Slides_aarch64.app.tar.gz Oku.Slides_x64.app.tar.gz Oku.Slides_${v}_amd64.AppImage
Oku.Slides_${v}_amd64.deb Oku.Slides_${v}_x64-setup.exe"
for name in $names; do
  printf 'fake bundle %s\n' "$name" > "$tmp/bundles/$name"
done

# Signs FILE with a Tauri key file (base64) and a trusted comment; prints the Tauri signature.
tauri_sig() {
  local keyfile=$1 password=$2 file=$3 comment=$4 dir
  dir=$(mktemp -d "$tmp/sig.XXXXXX")
  base64 -d < "$keyfile" > "$dir/secret.key"
  printf '%s\n' "$password" \
    | minisign -S -s "$dir/secret.key" -m "$file" -x "$dir/file.minisig" -t "$comment" > /dev/null 2>&1
  base64 < "$dir/file.minisig" | tr -d '\n'
}

# Writes a latest.json (v8.0.0's shape: base keys plus installer keys sharing URLs) to OUT,
# each bundle signed with KEYFILE and the trusted comment built by COMMENT_FMT (%s = name).
make_latest() {
  local out=$1 keyfile=$2 password=$3 comment_fmt=$4 sigs='{}' name s
  for name in $names; do
    # shellcheck disable=SC2059 # the format is the point
    s=$(tauri_sig "$keyfile" "$password" "$tmp/bundles/$name" "$(printf "$comment_fmt" "$name")")
    sigs=$(jq -c --arg n "$name" --arg s "$s" '. + {($n): $s}' <<< "$sigs")
  done
  jq -n --arg v "$v" --arg p "$prefix" --argjson sigs "$sigs" '
    def entry($n): {signature: $sigs[$n], url: ($p + $n)};
    { version: $v, notes: "Fake release notes.", pub_date: "2026-10-28T07:02:06.854Z",
      platforms: {
        "darwin-aarch64": entry("Oku.Slides_aarch64.app.tar.gz"),
        "darwin-aarch64-app": entry("Oku.Slides_aarch64.app.tar.gz"),
        "darwin-x86_64": entry("Oku.Slides_x64.app.tar.gz"),
        "darwin-x86_64-app": entry("Oku.Slides_x64.app.tar.gz"),
        "linux-x86_64": entry("Oku.Slides_\($v)_amd64.AppImage"),
        "linux-x86_64-appimage": entry("Oku.Slides_\($v)_amd64.AppImage"),
        "linux-x86_64-deb": entry("Oku.Slides_\($v)_amd64.deb"),
        "windows-x86_64": entry("Oku.Slides_\($v)_x64-setup.exe"),
        "windows-x86_64-nsis": entry("Oku.Slides_\($v)_x64-setup.exe") } }' > "$out"
}

run_sign() { # LATEST OUT [VERSION]
  TAURI_SIGNING_PRIVATE_KEY=$(cat "$tmp/real.key") TAURI_SIGNING_PRIVATE_KEY_PASSWORD="real pw" \
    "$sign" "$1" "$tmp/bundles" "${3:-$v}" "$repo" "$2"
}

expect_gate_ok() { # CASE LATEST [VERSION]
  if "$gate" "$2" "$tmp/bundles" "${3:-$v}" "$repo" "$real_pub" > "$tmp/out" 2>&1; then pass "$1"
  else fail "$1"; sed 's/^/    /' "$tmp/out"; fi
}

expect_gate_refuses() { # CASE LATEST PATTERN [BUNDLES]
  if "$gate" "$2" "${4:-$tmp/bundles}" "$v" "$repo" "$real_pub" > "$tmp/out" 2>&1; then
    fail "$1 (gate accepted)"
  elif grep -q -- "$3" "$tmp/out"; then pass "$1"
  else fail "$1 (refused for another reason)"; sed 's/^/    /' "$tmp/out"; fi
}

expect_sign_refuses() { # CASE LATEST PATTERN [VERSION]
  if run_sign "$2" "$tmp/refused" "${4:-$v}" > "$tmp/out" 2>&1; then fail "$1 (sign accepted)"
  elif grep -q -- "$3" "$tmp/out"; then pass "$1"
  else fail "$1 (refused for another reason)"; sed 's/^/    /' "$tmp/out"; fi
}

# The draft as the build jobs leave it: throwaway signatures, no version in the trusted comment.
make_latest "$tmp/draft.json" "$tmp/throwaway.key" "" 'timestamp:1\tfile:%s'

# Sign, then gate.
if run_sign "$tmp/draft.json" "$tmp/signed" > "$tmp/sigs" 2> "$tmp/out"; then
  pass "sign: draft re-signed with the real key"
else
  fail "sign: draft re-signed with the real key"; sed 's/^/    /' "$tmp/out"
fi
signed="$tmp/signed/latest.json"
expect_gate_ok "gate: re-signed latest.json accepted" "$signed"

expected_sigs=$(for name in $names; do echo "$name.sig"; done | sort)
if [ "$(sort "$tmp/sigs")" = "$expected_sigs" ] \
   && [ "$(for f in "$tmp/signed"/*.sig; do basename "$f"; done | sort)" = "$expected_sigs" ]; then
  pass "sign: one .sig per distinct bundle, listed on stdout"
else
  fail "sign: one .sig per distinct bundle, listed on stdout"; cat "$tmp/sigs"
fi
if jq -e --arg s "$(cat "$tmp/signed/Oku.Slides_${v}_x64-setup.exe.sig")" --slurpfile d "$tmp/draft.json" '
     .platforms["windows-x86_64"].signature == $s and .platforms["windows-x86_64-nsis"].signature == $s
     and .version == $d[0].version and .notes == $d[0].notes and .pub_date == $d[0].pub_date
     and ([.platforms[].url] == [$d[0].platforms[].url])
     and (.platforms | keys) == ($d[0].platforms | keys)' "$signed" > /dev/null; then
  pass "sign: shared URLs share the signature; keys, urls, notes, pub_date kept"
else
  fail "sign: shared URLs share the signature; keys, urls, notes, pub_date kept"
fi
comment=$(base64 -d < "$tmp/signed/Oku.Slides_aarch64.app.tar.gz.sig" | sed -n 's/^trusted comment: //p')
if printf '%s' "$comment" | grep -qE $'^timestamp:[0-9]+\tfile:Oku\\.Slides_aarch64\\.app\\.tar\\.gz\tversion:9\\.1\\.0$'; then
  pass "sign: trusted comment is timestamp, file and version"
else
  fail "sign: trusted comment is timestamp, file and version ($comment)"
fi

# The gate refuses what a compromised build, a stale release or a forged latest.json would give.
expect_gate_refuses "gate: throwaway-signed draft refused" "$tmp/draft.json" "does not verify"

make_latest "$tmp/control.json" "$tmp/real.key" "real pw" "timestamp:1\tfile:%s\tversion:$v"
expect_gate_ok "gate: control, real key and version:$v" "$tmp/control.json"
for case in "version:9.9.9|another version" "version:19.1.0|suffix trickery" \
            "version:9.1.00|prefix trickery" "version:v9.1.0|v prefix" \
            "version:9.1.0\tversion:9.9.9|two version fields" "|no version field"; do
  field=${case%%|*}
  make_latest "$tmp/forged.json" "$tmp/real.key" "real pw" "timestamp:1\tfile:%s${field:+\\t$field}"
  expect_gate_refuses "gate: ${case#*|} refused" "$tmp/forged.json" "trusted comment"
done

jq '.version = "9.9.9"' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: latest.json announcing another version refused" "$tmp/bad.json" "announces version"
jq --arg u "https://evil.example/releases/download/v$v/Oku.Slides_x64.app.tar.gz" \
  '.platforms["darwin-x86_64"].url = $u' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: foreign URL host refused" "$tmp/bad.json" "URL refused"
jq --arg u "https://github.com/$repo/releases/download/v8.0.0/Oku.Slides_x64.app.tar.gz" \
  '.platforms["darwin-x86_64-app"].url = $u' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: URL of another release refused" "$tmp/bad.json" "URL refused"
jq --arg u "${prefix}Oku..Slides_x64.app.tar.gz" \
  '.platforms["darwin-x86_64"].url = $u' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: URL with .. refused" "$tmp/bad.json" "URL refused"
jq 'del(.platforms["windows-x86_64"])' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: missing platform key refused" "$tmp/bad.json" "windows-x86_64 missing"
jq --arg s "$(cat "$tmp/signed/Oku.Slides_x64.app.tar.gz.sig")" \
  '.platforms["darwin-aarch64-app"].signature = $s' "$signed" > "$tmp/bad.json"
expect_gate_refuses "gate: signature of another bundle refused" "$tmp/bad.json" "does not verify"
cp -R "$tmp/bundles" "$tmp/tampered"
printf 'tampered\n' >> "$tmp/tampered/Oku.Slides_${v}_amd64.deb"
expect_gate_refuses "gate: tampered bundle refused" "$signed" "does not verify" "$tmp/tampered"

# release-sign.sh refuses to sign anything outside the release.
jq --arg u "https://evil.example/Oku.Slides_x64.app.tar.gz" \
  '.platforms["darwin-x86_64"].url = $u' "$tmp/draft.json" > "$tmp/bad.json"
expect_sign_refuses "sign: foreign URL refused" "$tmp/bad.json" "URL refused"
jq --arg u "${prefix}Oku..Slides_x64.app.tar.gz" \
  '.platforms["darwin-x86_64"].url = $u' "$tmp/draft.json" > "$tmp/bad.json"
expect_sign_refuses "sign: URL with .. refused" "$tmp/bad.json" "URL refused"
expect_sign_refuses "sign: version other than latest.json's refused" "$tmp/draft.json" \
  "announces version" 9.9.9
if TAURI_SIGNING_PRIVATE_KEY=$(cat "$tmp/real.key") TAURI_SIGNING_PRIVATE_KEY_PASSWORD="wrong" \
     "$sign" "$tmp/draft.json" "$tmp/bundles" "$v" "$repo" "$tmp/refused" > "$tmp/out" 2>&1; then
  fail "sign: wrong password refused (sign accepted)"
elif grep -q -- "could not sign" "$tmp/out"; then pass "sign: wrong password refused"
else fail "sign: wrong password refused (refused for another reason)"; sed 's/^/    /' "$tmp/out"; fi

if [ "$failures" -eq 0 ]; then echo "release-sign-test: all cases passed"; else
  echo "release-sign-test: $failures case(s) failed"; exit 1; fi
