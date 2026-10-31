# Shared by the release scripts (sourced, not run): the one rule for update bundle URLs.
#
# bundle_name PREFIX URL prints the asset name when URL is PREFIX plus one plain asset name
# (letters, digits, `._+-`, no `..`, neither latest.json nor a .sig), else fails. PREFIX is
# https://github.com/<repo>/releases/download/v<version>/. update-mirror.yml applies the same
# rule to the mirror.
bundle_name() {
  local prefix=$1 url=$2 name
  name=${url#"$prefix"}
  [[ "$name" != "$url" && "$name" =~ ^[A-Za-z0-9._+-]+$ && "$name" != *..* \
     && "$name" != latest.json && "$name" != *.sig ]] || return 1
  printf '%s\n' "$name"
}
