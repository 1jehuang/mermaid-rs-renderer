#!/usr/bin/env bash
#
# build-xcframework.sh — assemble MermaidFFI.xcframework from the ffi/ crate.
#
# Stages:
#   2. cbindgen + clang-format header-drift gate against the committed mmdr.h.
#   3. cross-compile the `mermaid-ffi` staticlib for five Apple triples with the
#      correct per-target deployment-version floor.
#   4. lipo the fat simulator + macOS slices and run
#      `xcodebuild -create-xcframework` over the three slices.
#   5. zip the framework + record its SHA-256.
#
# `mermaid-ffi` is a workspace member, and Cargo honors `[profile.*]` only at
# the workspace root. Setting the root profile would degrade the published CLI's
# release build, so the size/ABI knobs are applied here per-invocation via
# CARGO_PROFILE_RELEASE_* — affecting ONLY this xcframework build.

set -euo pipefail

# --- configuration -----------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FFI_CRATE_DIR="${REPO_ROOT}/ffi"
HEADER_DIR="${FFI_CRATE_DIR}/include" # carries mmdr.h + module.modulemap
HEADER_OUT="${HEADER_DIR}/mmdr.h"
TARGET_DIR="${REPO_ROOT}/target" # shared workspace target dir
BUILD_DIR="${REPO_ROOT}/.build/xcframework"
ARTIFACTS_DIR="${REPO_ROOT}/Artifacts"
XCFRAMEWORK="MermaidFFI.xcframework"
LIB_NAME="libmermaid_ffi.a"
CLANG_FORMAT_STYLE="file:${REPO_ROOT}/.clang-format"

# Per-slice deployment floors: mismatch = App Store rejection.
IOS_DEPLOYMENT_TARGET="16.0"
MACOS_DEPLOYMENT_TARGET="13.0"

# Five build slices: iOS-device single-arch, iOS-sim fat, macOS fat.
IOS_DEVICE_TARGET="aarch64-apple-ios"
IOS_SIM_TARGETS=("aarch64-apple-ios-sim" "x86_64-apple-ios")
MACOS_TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin")

# Release-profile knobs applied per-invocation (see header note). panic=unwind
# is MANDATORY: every extern "C" body relies on catch_unwind.
export CARGO_PROFILE_RELEASE_OPT_LEVEL="z"
export CARGO_PROFILE_RELEASE_LTO="true"
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1"
export CARGO_PROFILE_RELEASE_STRIP="symbols"
export CARGO_PROFILE_RELEASE_PANIC="unwind"

# --- helpers -----------------------------------------------------------------

log() { printf '\033[1;34m[build-xcframework]\033[0m %s\n' "$*"; }
die() {
	printf '\033[1;31m[build-xcframework] ERROR:\033[0m %s\n' "$*" >&2
	exit 1
}
require() { command -v "$1" >/dev/null 2>&1 || die "required tool '$1' not found on PATH"; }

deployment_env_for() {
	case "$1" in
	*-apple-ios | *-apple-ios-sim) echo "IPHONEOS_DEPLOYMENT_TARGET=${IOS_DEPLOYMENT_TARGET}" ;;
	*-apple-darwin) echo "MACOSX_DEPLOYMENT_TARGET=${MACOS_DEPLOYMENT_TARGET}" ;;
	*) die "unknown triple class: $1" ;;
	esac
}

build_target() { # build_target <triple>
	local triple="$1" env_kv
	env_kv="$(deployment_env_for "${triple}")"
	log "Building ${triple} (release, ${env_kv})..."
	(cd "${FFI_CRATE_DIR}" && env "${env_kv}" cargo build --release -p mermaid-ffi --target "${triple}")
	local lib="${TARGET_DIR}/${triple}/release/${LIB_NAME}"
	[ -f "${lib}" ] || die "expected staticlib not produced: ${lib}"
}

# --- preflight ---------------------------------------------------------------

[ -f "${FFI_CRATE_DIR}/Cargo.toml" ] || die "FFI crate missing at ${FFI_CRATE_DIR}"
require cargo
require rustc
require cbindgen
require clang-format
require lipo
require xcodebuild
require shasum

log "Repo root: ${REPO_ROOT}"

# --- stage 2: cbindgen + clang-format header-drift gate ----------------------

log "Checking header drift against committed ${HEADER_OUT}..."
HEADER_TMP="$(mktemp -d)/mmdr.h"
trap 'rm -rf "$(dirname "${HEADER_TMP}")"' EXIT
cbindgen --config "${FFI_CRATE_DIR}/cbindgen.toml" --crate mermaid-ffi --output "${HEADER_TMP}" "${FFI_CRATE_DIR}" 2>/dev/null
clang-format -i --style="${CLANG_FORMAT_STYLE}" "${HEADER_TMP}"
if ! diff -q "${HEADER_OUT}" "${HEADER_TMP}" >/dev/null; then
	die "cbindgen header drift: ${HEADER_OUT} differs from 'cbindgen | clang-format'. Regenerate (see ffi/cbindgen.toml) and bump abi_version if the layout changed."
fi
log "Header in sync."

# --- stage 3: cross-compile five slices --------------------------------------

log "Ensuring Rust targets are installed..."
rustup target add "${IOS_DEVICE_TARGET}" "${IOS_SIM_TARGETS[@]}" "${MACOS_TARGETS[@]}" >/dev/null

build_target "${IOS_DEVICE_TARGET}"
for t in "${IOS_SIM_TARGETS[@]}" "${MACOS_TARGETS[@]}"; do build_target "$t"; done

# --- stage 4: lipo fat slices + create-xcframework ---------------------------

log "Assembling slices..."
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/ios-device" "${BUILD_DIR}/ios-sim" "${BUILD_DIR}/macos"

cp "${TARGET_DIR}/${IOS_DEVICE_TARGET}/release/${LIB_NAME}" "${BUILD_DIR}/ios-device/${LIB_NAME}"
lipo -create \
	"${TARGET_DIR}/aarch64-apple-ios-sim/release/${LIB_NAME}" \
	"${TARGET_DIR}/x86_64-apple-ios/release/${LIB_NAME}" \
	-output "${BUILD_DIR}/ios-sim/${LIB_NAME}"
lipo -create \
	"${TARGET_DIR}/aarch64-apple-darwin/release/${LIB_NAME}" \
	"${TARGET_DIR}/x86_64-apple-darwin/release/${LIB_NAME}" \
	-output "${BUILD_DIR}/macos/${LIB_NAME}"

log "Running xcodebuild -create-xcframework..."
rm -rf "${BUILD_DIR}/${XCFRAMEWORK}"
xcodebuild -create-xcframework \
	-library "${BUILD_DIR}/ios-device/${LIB_NAME}" -headers "${HEADER_DIR}" \
	-library "${BUILD_DIR}/ios-sim/${LIB_NAME}" -headers "${HEADER_DIR}" \
	-library "${BUILD_DIR}/macos/${LIB_NAME}" -headers "${HEADER_DIR}" \
	-output "${BUILD_DIR}/${XCFRAMEWORK}"

# --- stage 5: zip + sha256 ---------------------------------------------------

mkdir -p "${ARTIFACTS_DIR}"
(cd "${BUILD_DIR}" && ditto -c -k --keepParent "${XCFRAMEWORK}" "${ARTIFACTS_DIR}/${XCFRAMEWORK}.zip")
shasum -a 256 "${ARTIFACTS_DIR}/${XCFRAMEWORK}.zip" | awk '{print $1}' >"${ARTIFACTS_DIR}/${XCFRAMEWORK}.zip.sha256"
log "Artifact: ${ARTIFACTS_DIR}/${XCFRAMEWORK}.zip"
log "SHA-256:  $(cat "${ARTIFACTS_DIR}/${XCFRAMEWORK}.zip.sha256")"

log "Build complete. Slice sizes:"
du -h "${BUILD_DIR}/ios-device/${LIB_NAME}" "${BUILD_DIR}/ios-sim/${LIB_NAME}" "${BUILD_DIR}/macos/${LIB_NAME}" | sed 's/^/  /'
