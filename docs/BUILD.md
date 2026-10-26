# Build and install ScanCraft

Linux is the supported development platform. Install Rust using rustup; the
repository pins its tested toolchain in `rust-toolchain.toml`. On Debian/Ubuntu:

```sh
sudo apt-get install libclang-dev pkg-config libwayland-dev libxkbcommon-dev libgl1-mesa-dev
cargo build --locked --release --workspace
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
./target/release/scancraft
./target/release/scancraft-cli discover
install -Dm755 target/release/scancraft "$HOME/.local/bin/scancraft"
install -Dm755 target/release/scancraft-cli "$HOME/.local/bin/scancraft-cli"
```

Open an existing PLY or editable project by passing its path to `scancraft`.
The desktop opens without acquiring the scanner; acquisition is explicit.
Camera access requires appropriate device permissions. The development scanner
uses a workstation-specific Y14/metadata kernel configuration. Unmodified kernels,
other hardware and fresh-machine acquisition are not qualified. Do not install
experimental kernel patches as a routine application installation step.

Reference numerical tests under `tools/` may require separately installed Python
packages listed in their requirements files. Proprietary DLL/GPU oracle probes
require private user-supplied software and are not ordinary CI or runtime dependencies.
