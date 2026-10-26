# Initial unsafe-code review

Reviewed scope: native Linux development build on x86_64. This is an internal
review, not an independent memory-safety audit or completed release gate.

- Linux structs are generated from installed UAPI headers by v4l2-sys-mit.
  Calls use the matching request number, direction and ABI size. Device fds
  are owned by Stream. `zeroed` is limited to C POD structs with integer,
  union and byte-array members; zero is valid for these representations.
- mmap offsets/lengths come from QUERYBUF. Length is bounded at 64 MiB. Each
  mapping is owned and unmapped once. Raw slices exist only after DQBUF,
  are bounded by bytesused <= mapping length, are copied before QBUF, and
  never escape as borrowed device memory. Unsafe pointers never cross the
  acquisition worker boundary.
- poll references one initialized pollfd and a live owned fd. ioctl retries
  EINTR. poll returns interruption as an error and still runs RAII cleanup.
- Explicit close and Drop stop streams, unmap, release request buffers,
  restore camera format/rate and close fds. Explicit close reports failures.
- flock/getuid receive valid owned file descriptors or require no arguments.

Remaining work: independently validate ioctl ABI constants on each supported
architecture; signal/disconnect failure injection; memory-safety tooling and
review by a second reviewer. No transitive dependency audit has been completed.

The read-only XU query uses a repr(C) pointer-bearing UAPI struct. Its buffer
is bounded at 1024 bytes, remains alive through ioctl, and the response parser
validates magic/opcode/id/status and halfword lengths before returning bytes.
The nusb experiment uses safe library APIs but failed device recovery; Rust
memory safety does not establish device-protocol safety or correct recovery.
