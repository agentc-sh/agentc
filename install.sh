#!/usr/bin/env bash
set -euo pipefail

SITE_HOST="${AGENTC_SITE_HOST:-agentc.sh}"
PACKAGE_NAME="agentc"
BINARY_NAME="agentc"

API_BASE="https://${SITE_HOST}/api/binary"

TMP_DIR=""

ARG_VERSION=""
ARG_PRE=false
ARG_INSTALL_DIR=""
ARG_TOKEN=""
ARG_NO_VERIFY=false
ARG_NO_CHECKSUM=false
ARG_DRY_RUN=false
ARG_UPGRADE=false
ARG_QUIET=false

# ==============================================================================
# COLOURS
# ==============================================================================

if [ -t 1 ]; then
  COL_RED="\033[0;31m"
  COL_YELLOW="\033[0;33m"
  COL_GREEN="\033[0;32m"
  COL_CYAN="\033[0;36m"
  COL_BOLD="\033[1m"
  COL_RESET="\033[0m"
else
  COL_RED="" COL_YELLOW="" COL_GREEN="" COL_CYAN="" COL_BOLD="" COL_RESET=""
fi

# ==============================================================================
# HELPERS
# ==============================================================================

log()     { [[ "$ARG_QUIET" == "true" ]] || printf "%b\n" "$*"; }
info()    { log "${COL_CYAN}==>${COL_RESET} $*"; }
ok()      { log "${COL_GREEN}✔${COL_RESET}  $*"; }
warn()    { printf "%b\n" "${COL_YELLOW}warn:${COL_RESET} $*" >&2; }
die()     { printf "%b\n" "${COL_RED}error:${COL_RESET} $*" >&2; exit 1; }
dry_log() { printf "%b\n" "${COL_YELLOW}dry-run:${COL_RESET} $*"; }

need() {
  command -v "$1" &>/dev/null || die "Required tool not found: $1. Please install it and retry."
}

# ==============================================================================
# VALIDATION
# ==============================================================================

validate_args() {
  [[ -n "$ARG_INSTALL_DIR" ]] || die "--install-dir is required."
  [[ -n "$ARG_TOKEN" ]] || die "--token is required. Provide your agentc.sh access token."
}

usage() {
  cat <<EOF
Usage:
  curl -sSfL https://install.${SITE_HOST} | bash -s -- [OPTIONS]

Options:
  -v, --version <version>     Version to install (default: latest stable)
      --pre                   Allow pre-release versions
  -d, --install-dir <path>    Directory to install binary into (required)
  -t, --token <token>         agentc.sh access token (required)
      --no-verify             Skip signature and provenance verification
      --no-checksum           Skip SHA256 checksum verification
      --dry-run               Print what would be done without downloading or installing
      --upgrade               Replace existing installation if present
  -q, --quiet                 Suppress non-error output
  -h, --help                  Show this help message

Examples:
  # Install latest stable
  curl -sSfL https://install.${SITE_HOST} | bash -s -- \\
    --install-dir /usr/local/bin \\
    --token <your-token>

  # Install a specific version
  curl -sSfL https://install.${SITE_HOST} | bash -s -- \\
    --install-dir /usr/local/bin \\
    --token <your-token> \\
    --version 1.2.3

  # Install latest pre-release
  curl -sSfL https://install.${SITE_HOST} | bash -s -- \\
    --install-dir /usr/local/bin \\
    --token <your-token> \\
    --pre

  # Upgrade an existing installation
  curl -sSfL https://install.${SITE_HOST} | bash -s -- \\
    --install-dir /usr/local/bin \\
    --token <your-token> \\
    --upgrade

  # Dry run — see what would happen without installing
  curl -sSfL https://install.${SITE_HOST} | bash -s -- \\
    --install-dir /usr/local/bin \\
    --token <your-token> \\
    --dry-run
EOF
  exit 0
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -v|--version)     ARG_VERSION="${2:?--version requires a value}";         shift 2 ;;
      --pre)            ARG_PRE=true;                                            shift   ;;
      -d|--install-dir) ARG_INSTALL_DIR="${2:?--install-dir requires a value}"; shift 2 ;;
      -t|--token)       ARG_TOKEN="${2:?--token requires a value}";              shift 2 ;;
      --no-verify)      ARG_NO_VERIFY=true;                                      shift   ;;
      --no-checksum)    ARG_NO_CHECKSUM=true;                                    shift   ;;
      --dry-run)        ARG_DRY_RUN=true;                                        shift   ;;
      --upgrade)        ARG_UPGRADE=true;                                        shift   ;;
      -q|--quiet)       ARG_QUIET=true;                                          shift   ;;
      -h|--help)        usage ;;
      *) echo "error: unknown option: $1" >&2; exit 1 ;;
    esac
  done
}

# ==============================================================================
# PLATFORM DETECTION
# ==============================================================================

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)               OS="linux"   ;;
    MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
    *) die "Unsupported operating system: $os" ;;
  esac

  case "$arch" in
    x86_64|amd64) ARCH="x86_64" ;;
    *) die "Unsupported architecture: $arch (only x86_64 is supported)" ;;
  esac

  if [[ "$OS" == "windows" ]]; then
    BINARY_FILENAME="${BINARY_NAME}-windows-x86_64.exe"
    INSTALL_BINARY="${BINARY_NAME}.exe"
  else
    BINARY_FILENAME="${BINARY_NAME}-linux-x86_64"
    INSTALL_BINARY="${BINARY_NAME}"
  fi

  CHECKSUM_FILENAME="${BINARY_FILENAME}.sha256"
  SIG_BUNDLE_FILENAME="${BINARY_FILENAME}.sigstore.json"
  ATTESTATION_FILENAME="${BINARY_FILENAME}.attestation.json"

  log "${COL_BOLD}Platform:${COL_RESET} ${OS}/${ARCH}"
}

# ==============================================================================
# VERSION RESOLUTION
# ==============================================================================

resolve_version() {
  if [[ -n "$ARG_VERSION" ]]; then
    RESOLVED_VERSION="$ARG_VERSION"
    log "${COL_BOLD}Version:${COL_RESET}  ${RESOLVED_VERSION} (pinned)"
    return
  fi

  info "Fetching available versions..."

  local url="${API_BASE}/versions"
  [[ "$ARG_PRE" == "true" ]] && url="${url}?pre=true"

  local raw
  raw=$(curl \
    --fail --silent --show-error --location \
    --header "Authorization: Bearer ${ARG_TOKEN}" \
    "$url" \
  ) || die "Failed to fetch version list.\nCheck your token and ensure you have access."

  local versions
  versions=$(printf '%s' "$raw" | grep -o '"[^"]*"' | tr -d '"')
  [[ -n "$versions" ]] || die "No versions found."

  RESOLVED_VERSION=$(printf '%s\n' $versions | sort -V | tail -n1)
  log "${COL_BOLD}Version:${COL_RESET}  ${RESOLVED_VERSION}"
}

# ==============================================================================
# DOWNLOAD
# ==============================================================================

download_file() {
  local file="$1" dest="$2" label="$3"
  local url="${API_BASE}/download?version=${RESOLVED_VERSION}&file=${file}"
  info "Downloading ${label}..."
  curl \
    --fail --silent --show-error --location \
    --progress-bar \
    --header "Authorization: Bearer ${ARG_TOKEN}" \
    --output "$dest" \
    "$url" || die "Failed to download ${label}."
}

download_attestation() {
  local file="$1" dest="$2"
  local url="${API_BASE}/attestation?version=${RESOLVED_VERSION}&file=${file}"

  info "Downloading attestation data..."

  curl \
    --fail --silent --show-error --location \
    --header "Authorization: Bearer ${ARG_TOKEN}" \
    --output "$dest" \
    "$url" || die "Failed to download attestation data."
}

# ==============================================================================
# VERIFICATION
# ==============================================================================

verify_checksum() {
  local binary="$1" checksum_file="$2"
  info "Verifying checksum..."
  local expected actual
  expected=$(tr -d '[:space:]' < "$checksum_file")
  actual=$(sha256sum "$binary" | awk '{print $1}')
  [[ "$expected" == "$actual" ]] || die "Checksum mismatch!\n  Expected: ${expected}\n  Got:      ${actual}"
  ok "Checksum verified."
}

json_has_available_true() {
  local file="$1"

  grep -Eq '"available"[[:space:]]*:[[:space:]]*true' "$file"
}

json_extract_string() {
  local key="$1" file="$2"

  awk -v key="\"${key}\":\"" '
    {
      json = json $0
    }

    END {
      start = index(json, key)

      if (!start) {
        exit 1
      }

      pos = start + length(key)
      out = ""
      esc = 0

      for (i = pos; i <= length(json); i += 1) {
        ch = substr(json, i, 1)

        if (esc) {
          out = out ch
          esc = 0
          continue
        }

        if (ch == "\\") {
          out = out ch
          esc = 1
          continue
        }

        if (ch == "\"") {
          print out
          exit 0
        }

        out = out ch
      }

      exit 1
    }
  ' "$file"
}

json_extract_object() {
  local key="$1" file="$2"

  awk -v key="\"${key}\":" '
    {
      json = json $0
    }

    END {
      start = index(json, key)

      if (!start) {
        exit 1
      }

      pos = start + length(key)

      while (substr(json, pos, 1) ~ /[[:space:]]/) {
        pos += 1
      }

      if (substr(json, pos, 1) != "{") {
        exit 1
      }

      out = ""
      depth = 0
      in_string = 0
      esc = 0

      for (i = pos; i <= length(json); i += 1) {
        ch = substr(json, i, 1)
        out = out ch

        if (in_string) {
          if (esc) {
            esc = 0
            continue
          }

          if (ch == "\\") {
            esc = 1
            continue
          }

          if (ch == "\"") {
            in_string = 0
          }

          continue
        }

        if (ch == "\"") {
          in_string = 1
          continue
        }

        if (ch == "{") {
          depth += 1
          continue
        }

        if (ch == "}") {
          depth -= 1

          if (depth == 0) {
            print out
            exit 0
          }
        }
      }

      exit 1
    }
  ' "$file"
}

write_decoded_json_string() {
  local value="$1" dest="$2"

  value="${value//\\\//\/}"
  value="${value//\\\"/\"}"
  value="${value//\\r/$'\r'}"
  value="${value//\\n/$'\n'}"
  value="${value//\\t/$'\t'}"
  value="${value//\\\\/\\}"

  printf '%s' "$value" > "$dest"
}

verify_signature() {
  local binary="$1" bundle="$2" attestation_response="$3"
  local signature_identity signature_oidc_issuer

  info "Verifying keyless cosign signature..."

  need cosign

  signature_identity="$(json_extract_string signatureIdentity "$attestation_response")" \
    || die "Failed to parse signature identity."
  signature_oidc_issuer="$(json_extract_string signatureOidcIssuer "$attestation_response")" \
    || die "Failed to parse signature OIDC issuer."

  if cosign verify-blob \
    --bundle "$bundle" \
    --certificate-identity-regexp "$signature_identity" \
    --certificate-oidc-issuer "$signature_oidc_issuer" \
    "$binary" >/dev/null 2>&1; then
    ok "Signature verified."
  else
    die "Signature verification failed. The binary may have been tampered with."
  fi
}

verify_provenance() {
  local binary="$1" attestation_response="$2"
  local repository bundle_file_name trusted_root_file_name trusted_root_raw
  local bundle_path trusted_root_path

  if ! json_has_available_true "$attestation_response"; then
    warn "No valid build attestation is available for this artifact. Skipping provenance verification."
    return
  fi

  need gh

  repository="$(json_extract_string repository "$attestation_response")" \
    || die "Failed to parse attestation repository."
  bundle_file_name="$(json_extract_string bundleFileName "$attestation_response")" \
    || die "Failed to parse attestation bundle filename."
  trusted_root_file_name="$(json_extract_string trustedRootFileName "$attestation_response")" \
    || die "Failed to parse attestation trusted root filename."
  trusted_root_raw="$(json_extract_string trustedRoot "$attestation_response")" \
    || die "Failed to parse attestation trusted root."

  bundle_path="${TMP_DIR}/${bundle_file_name}"
  trusted_root_path="${TMP_DIR}/${trusted_root_file_name}"

  json_extract_object bundle "$attestation_response" > "$bundle_path" \
    || die "Failed to parse attestation bundle."
  write_decoded_json_string "$trusted_root_raw" "$trusted_root_path"

  info "Verifying build provenance..."

  if gh attestation verify \
    "$binary" \
    --repo "$repository" \
    --bundle "$bundle_path" \
    --custom-trusted-root "$trusted_root_path" >/dev/null 2>&1; then
    ok "Build provenance verified."
  else
    die "Build provenance verification failed. The binary may not match the published GitHub build attestation."
  fi
}

# ==============================================================================
# INSTALLATION
# ==============================================================================

install_binary() {
  local src="$1" dest_dir="$2" dest_name="$3"
  local dest="${dest_dir}/${dest_name}"
  info "Installing to ${dest}..."
  if [[ ! -d "$dest_dir" ]]; then
    mkdir -p "$dest_dir" || die "Failed to create install directory: ${dest_dir}"
  fi
  if [[ -e "$dest" ]]; then
    info "Upgrading existing installation at ${dest}..."
  fi
  cp "$src" "$dest" || die "Failed to copy binary to ${dest}.\nYou may need elevated permissions."
  chmod +x "$dest"  || die "Failed to set executable permission on ${dest}."
  ok "Installed ${dest_name} to ${dest}."
}

check_path() {
  local install_dir="$1"
  if ! echo ":${PATH}:" | grep -q ":${install_dir}:"; then
    warn "${install_dir} is not on your PATH."
    warn "Add it to your shell profile:"
    warn "  export PATH=\"${install_dir}:\$PATH\""
  fi
}

# ==============================================================================
# DRY RUN
# ==============================================================================

print_dry_run_summary() {
  log ""
  log "${COL_BOLD}${COL_YELLOW}Dry run — no files will be downloaded or installed.${COL_RESET}"
  log "────────────────────────────────────────────────────"
  dry_log "Binary:        ${BINARY_FILENAME}"
  dry_log "Version:       ${RESOLVED_VERSION}"
  dry_log "Platform:      ${OS}/${ARCH}"
  dry_log "Install path:  ${ARG_INSTALL_DIR}/${INSTALL_BINARY}"
  log ""
  dry_log "Would download via ${API_BASE}/download?version=${RESOLVED_VERSION}&file=<filename>:"
  dry_log "  ${BINARY_FILENAME}"
  if [[ "$ARG_NO_CHECKSUM" != "true" ]]; then
    dry_log "  ${CHECKSUM_FILENAME}"
  fi
  if [[ "$ARG_NO_VERIFY" != "true" ]]; then
    dry_log "  ${SIG_BUNDLE_FILENAME}"
    dry_log "  ${ATTESTATION_FILENAME} (via ${API_BASE}/attestation?version=${RESOLVED_VERSION}&file=${BINARY_FILENAME})"
  fi
  log ""
  dry_log "Would verify:"
  if [[ "$ARG_NO_CHECKSUM" == "true" ]]; then
    dry_log "  checksum:  skipped (--no-checksum)"
  else
    dry_log "  checksum:  sha256sum ${BINARY_FILENAME} == ${CHECKSUM_FILENAME}"
  fi
  if [[ "$ARG_NO_VERIFY" == "true" ]]; then
    dry_log "  signature:  skipped (--no-verify)"
    dry_log "  provenance: skipped (--no-verify)"
  elif command -v cosign &>/dev/null; then
    dry_log "  signature:  cosign verify-blob --bundle ${SIG_BUNDLE_FILENAME} ${BINARY_FILENAME}"
  else
    dry_log "  signature:  requires cosign on PATH"
  fi
  if [[ "$ARG_NO_VERIFY" == "true" ]]; then
    :
  elif command -v gh &>/dev/null; then
    dry_log "  provenance: gh attestation verify ${BINARY_FILENAME} --repo <repository> --bundle <bundle> --custom-trusted-root <trusted_root>"
  else
    dry_log "  provenance: requires gh on PATH when attestation data is available"
  fi
  log ""
  dry_log "Would install:"
  if [[ -e "${ARG_INSTALL_DIR}/${INSTALL_BINARY}" ]]; then
    if [[ "$ARG_UPGRADE" == "true" ]]; then
      dry_log "  (upgrading existing installation)"
    else
      dry_log "  (would fail — ${ARG_INSTALL_DIR}/${INSTALL_BINARY} already exists, use --upgrade)"
    fi
  fi
  dry_log "  cp ${BINARY_FILENAME} ${ARG_INSTALL_DIR}/${INSTALL_BINARY}"
  dry_log "  chmod +x ${ARG_INSTALL_DIR}/${INSTALL_BINARY}"
  if ! echo ":${PATH}:" | grep -q ":${ARG_INSTALL_DIR}:"; then
    log ""
    dry_log "Note: ${ARG_INSTALL_DIR} is not currently on your PATH."
  fi
  log ""
  log "${COL_YELLOW}Dry run complete. Re-run without --dry-run to install.${COL_RESET}"
  log ""
}

# ==============================================================================
# MAIN
# ==============================================================================

main() {
  parse_args "$@"
  validate_args

  log ""
  log "${COL_BOLD}agentc installer${COL_RESET}"
  log "────────────────────────────────"

  need curl
  need sha256sum
  need sort

  detect_platform

  # Check for existing installation before hitting the network at all
  if [[ -e "${ARG_INSTALL_DIR}/${INSTALL_BINARY}" ]] && [[ "$ARG_UPGRADE" != "true" ]]; then
    die "${ARG_INSTALL_DIR}/${INSTALL_BINARY} already exists. Use --upgrade to replace it."
  fi

  resolve_version

  if [[ "$ARG_DRY_RUN" == "true" ]]; then
    print_dry_run_summary
    exit 0
  fi

  TMP_DIR=$(mktemp -d)
  trap 'rm -rf "$TMP_DIR"' EXIT

  local tmp_binary="${TMP_DIR}/${BINARY_FILENAME}"
  local tmp_checksum="${TMP_DIR}/${CHECKSUM_FILENAME}"
  local tmp_bundle="${TMP_DIR}/${SIG_BUNDLE_FILENAME}"
  local tmp_attestation="${TMP_DIR}/${ATTESTATION_FILENAME}"

  download_file "$BINARY_FILENAME" "$tmp_binary" "$BINARY_FILENAME"

  if [[ "$ARG_NO_CHECKSUM" == "true" ]]; then
    warn "Checksum verification skipped (--no-checksum)."
  else
    download_file "$CHECKSUM_FILENAME" "$tmp_checksum" "$CHECKSUM_FILENAME"
    verify_checksum "$tmp_binary" "$tmp_checksum"
  fi

  if [[ "$ARG_NO_VERIFY" == "true" ]]; then
    warn "Signature and provenance verification skipped (--no-verify)."
  else
    download_attestation "$BINARY_FILENAME" "$tmp_attestation"
    download_file "$SIG_BUNDLE_FILENAME" "$tmp_bundle" "$SIG_BUNDLE_FILENAME"
    verify_signature "$tmp_binary" "$tmp_bundle" "$tmp_attestation"
    verify_provenance "$tmp_binary" "$tmp_attestation"
  fi

  install_binary "$tmp_binary" "$ARG_INSTALL_DIR" "$INSTALL_BINARY"
  check_path "$ARG_INSTALL_DIR"

  log ""
  log "${COL_GREEN}${COL_BOLD}Done!${COL_RESET} ${BINARY_NAME} ${RESOLVED_VERSION} installed successfully."
  log ""

  if command -v "${ARG_INSTALL_DIR}/${INSTALL_BINARY}" &>/dev/null; then
    "${ARG_INSTALL_DIR}/${INSTALL_BINARY}" --version 2>/dev/null || true
  fi
}

main "$@"
