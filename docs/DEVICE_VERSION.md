# Native device-version query

`scancraft-cli device-version` reads the connected Raptor's reported firmware and
hardware strings. This supplies a missing observation for later calibration
binding and firmware comparisons; it does not authenticate a device or establish
physical calibration. `protocol-version` remains the separate raw host-protocol
query. USB `bcdDevice` is not substituted for firmware version.

## Protocol and implementation

`scancraft_device::control::device_version::read` requires the caller's
`SessionLock`. The existing `ReadChannel` discovers exactly one Raptor interface0,
checks its extension-unit GUID and 64-byte request-channel length, and holds the
video-fd advisory lock. The new command also checks the response channel length
before sending anything beyond those GET_LEN requests.

The fixed request is GET_STRUCTURE_DATA opcode3, property1000, a two-halfword
payload and a matching request ID. UVC SET_CUR on selector2 transports the
zero-padded 64-byte read envelope. GET_CUR on selector1 receives 512 bytes.
Existing response validation checks magic, opcode, request ID, status and declared
length. The decoder accepts only the observed 164-byte version payload. No generic
property setter, firmware updater or arbitrary-memory reader is exposed.

Fields have bounded 16-byte, NUL-terminated printable ASCII values; bytes after
the first NUL are padding. Firmware must be nonempty. Other empty fields remain
empty, and the device-type integer is retained without inventing an enum meaning.
Reserved bytes are uninterpreted. Malformed text, unsupported sizes and transport
failures remain errors. Reported serial values may be placeholders; both source
and calibration binding flags remain false.

This is an explicit CLI operation. Automatic acquisition observations still
report firmware as not queried; the change does not add concurrent XU requests
inside the camera workers. Device-read strings alone must not silently populate
an authenticated or physically qualified calibration binding. Driver ioctl and
cleanup deadline limitations remain as documented in SCAN-REV-005.

## Source provenance

Independently written Rust, using protocol/layout references from MIT-licensed
OrbbecSDK_v2 at revision `85e94c4ee4032817df2e33228dbd7f51517d3d4e`:

- [Protocol.cpp::initGetStructureDataReq](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/protocol/Protocol.cpp)
- [InternalProperty.hpp::OB_STRUCT_VERSION](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/device/component/property/InternalProperty.hpp)
- [InternalTypes.hpp::OBVersionInfo](https://github.com/orbbec/OrbbecSDK_v2/blob/85e94c4ee4032817df2e33228dbd7f51517d3d4e/src/shared/InternalTypes.hpp)

The local checkout's referenced files were unmodified. The public SDK reference
alone does not prove Raptor compatibility: the exact read was independently
observed on the connected Raptor before adding the command. An existing private
Python transport supplied the first reference reply; no proprietary implementation
was copied. No SDK/runtime dependency or library version changed. The upstream
MIT notice is retained under THIRD_PARTY_LICENSES/OrbbecSDK_v2-protocol-reference/.
This does not import or license the SDK's separately licensed extension binaries.

## Verified hardware and test evidence

On 2026-11-03 the native command returned identical decoded results in three
consecutive reads, matching all decoded fields of the retained 164-byte raw
reference response. The connected scanner reports firmware **1.5.7**. Therefore
the separately downloaded official V1.4.8 package is older than the installed
reported version; no update or downgrade was attempted. Version text is not a
cryptographic attestation or evidence of what specific fixes are installed.

A subsequent native stereo capture retained ten frames per camera. Both full
recording replays passed. Left/right startup rejects were0/1; ongoing rejects
and missing metadata were0/0. These counters remain in the retained reports.
Reported stream state on interfaces0/2/4 and all queried
illumination state matched before and after the reads plus capture. This is a
short communication/restoration check, not sustained reliability qualification.

Three new synthetic parser/protocol tests cover every truncated payload length,
extended layouts, all text-field termination/encoding failures, empty firmware,
raw integer/field offsets, padding/reserved handling, exact fixed request bytes,
and incorrect response magic/opcode/ID/status/length. No device identifiers or
vendor payloads appear in fixtures. Full tools/check.sh passed formatting, strict
all-target Clippy, 835 tests (one existing ignored), and optimized builds.

Private evidence `native-device-version-1btnvpiw` retains reference request/reply,
parsed results, before/after snapshots, actual captures, replay logs, source and
executable hashes. No raw device data or firmware package is published.
