# Dictation API: custom headers and query parameters — Design

Task: t-1213. Builds on `2026-10-23-dictee-api-design.md` and `2026-10-23-reglages-dictee-a2ui-design.md`.

## Problem

Azure OpenAI answers **400** to the dictation request: it requires an `api-version` query
parameter, which `normalized_url` refuses in the address, and it expects the key in an
`api-key` header rather than `Authorization: Bearer`. Other gateways need custom headers too
(the Python SDK equivalents: `api_version=`, `default_headers=`). The 400 was also hard to
diagnose: only the status is logged.

## Design

- **Generic, no Azure mode.** `ApiConfig` gains:
  - `query: Vec<ApiParam { name, value }>`: appended to `…/audio/transcriptions`, URL-encoded
    by `url`. Stored and shown in clear (not secret).
  - `headers: Vec<ApiHeader { name, value: HeaderValue }>`, where
    `HeaderValue = Literal(String) | Env(String)`. A literal is stored and shown; an `Env`
    names a variable read only in `ApiTranscriber`, like the key: never stored, logged or
    sent to the UI (the UI shows the name and "found / not set").
- **Bearer becomes optional:** an empty `key_env` sends no `Authorization` header, but some
  variable must still be named (the key or an `Env` header). The API model counts as
  installed when every named variable is set (`key_present`); `Status::api_missing` names the
  first one unset for the hints.
- **Validation (`api_extra`).** On every write (`valid_api_headers`, per field, since the box
  saves one field at a time): header names are HTTP tokens; refused: `Host`, `Content-Type`,
  `Content-Length`, `Transfer-Encoding`, `Connection`, `Proxy-Authorization`, `TE`, `Trailer`,
  `Upgrade`, `Expect`, `Keep-Alive`; no duplicate names. Literals: printable ASCII and space,
  no CR/LF. Param names/values: visible ASCII without `&#%`, non-empty name. At most 8
  headers and 8 params, 256 bytes per value. At use and before the agent's switch to the API
  (`valid_api_access`, rules across fields): a variable named, `Authorization` only without a
  key variable.
- **User-only, like `url` and `key_env` (t-1144):** `headers` and `query` decide which secret
  goes where, so `set_dictation` refuses them before anything is applied; the agent never
  sees an `Env` header's value. Its confirmation before switching to the API lists every
  variable whose secret will be sent.
- **Settings box:** two single-line fields after the key variable, in a compact syntax
  parsed by `api_extra` (refused when invalid, like the URL), plus an Azure hint:
  - "Query parameters": `api-version=2025-03-01-preview&foo=bar`
  - "Extra headers": `api-key: $AZURE_OPENAI_API_KEY; X-Tenant: acme` (`$NAME` = variable;
    a literal cannot contain `;`).
- **Diagnosis:** on a non-2xx other than 401/403, log and show the API's `error.message`
  (JSON body), cleaned of control characters, truncated to 200 chars. Masked: every value read
  from a variable, each of its words of 8+ characters, and literals of 8+ characters. No
  message at all if a secret is under 8 characters or 8 characters of one remain (partial echo).

Azure example: URL `https://<res>.openai.azure.com/openai/deployments/<deployment>`, key
variable empty, query `api-version=2025-03-01-preview`, headers `api-key: $AZURE_OPENAI_API_KEY`.

## Out of scope

Entra ID (OAuth) tokens; per-request header templating.

## Testing

`api_check` rules (rstest tables), the request against the `tiny_http` fake (query, headers,
no Bearer when `key_env` is empty), redaction of the error message, MCP refusal regression
(`issue_1213_…`), the settings box fields, a non-English localized error.
