# Security reporting

Do not post credentials, private scans, device identifiers or proprietary binaries
in public issues. Use GitHub private vulnerability reporting if enabled; otherwise
contact the repository owner privately through an established channel before sharing
sensitive details. There is no promised response SLA or supported stable release yet.

Treat input files as untrusted. Bounded parsing and CRC checks reduce accidental
corruption risks; CRC is not authentication. Scanner tools access Linux devices and
are experimental. Do not run ordinary application workflows as root.
