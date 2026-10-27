# Dependencies and update strategy

Audited source: `ced7697fa0a4e2a262ae255efffa53ca577f117f`. Labels and scope are defined in ARCHITECTURE.md.

## Confirmed graph and licensing evidence

`Cargo.toml` declares 12 local workspace crates, edition 2024, application MIT
license and broad registry version requirements. `Cargo.lock` pins **338 external
package/version records**. All 338 exactly match `docs/publication/DEPENDENCIES.json`;
every listed retained notice path exists in `THIRD_PARTY_LICENSES/`. This is a
reproducible inventory/notice-path check, not an independent legal verification of
every upstream copyright or platform binary. The full transitive table follows.
Upstream revision fields below are reported metadata from that inventory, not
proof that a package tarball exactly equals an upstream Git checkout.

`THIRD_PARTY_NOTICES.md` separates MIT workspace code, third-party licenses/fonts
and GPL-2.0-only experimental kernel material. Four package/version records have
no retained standalone notice file: dispatch 0.2.0, hexf-parse 0.2.1, r-efi 5.3.0
and r-efi 6.0.0. The existing publication record says these are inactive for the
Linux GUI and not vendored; other-platform/binary packaging must recheck its
selected graph and resolve notices. A complete lock graph is not a linked-binary
bill of materials. See publication/PUBLICATION_AUDIT.md and THIRD_PARTY_NOTICES.md.

## Direct external dependencies by crate

- [scancraft-acquisition](../../crates/scancraft-acquisition/Cargo.toml): serde_json 1.0.154.
- [scancraft-app](../../crates/scancraft-app/Cargo.toml): serde_json 1.0.154; eframe 0.33.3; image 0.25.10; libc 0.2.191; nalgebra 0.35.0.
- [scancraft-calibration](../../crates/scancraft-calibration/Cargo.toml): serde 1.0.229; serde_json 1.0.154; crc32fast 1.5.3; nalgebra 0.35.0.
- [scancraft-cli](../../crates/scancraft-cli/Cargo.toml): serde_json 1.0.154; libc 0.2.191; image 0.25.10; serde 1.0.229; crc32fast 1.5.3.
- [scancraft-core](../../crates/scancraft-core/Cargo.toml): serde 1.0.229.
- [scancraft-depth](../../crates/scancraft-depth/Cargo.toml): serde 1.0.229.
- [scancraft-device](../../crates/scancraft-device/Cargo.toml): libc 0.2.191; v4l2-sys-mit 0.3.0; serde 1.0.229.
- [scancraft-geometry](../../crates/scancraft-geometry/Cargo.toml): serde 1.0.229; serde_json 1.0.154; nalgebra 0.35.0; crc32fast 1.5.3.
- [scancraft-image](../../crates/scancraft-image/Cargo.toml): local workspace crates only.
- [scancraft-recording](../../crates/scancraft-recording/Cargo.toml): serde_json 1.0.154; crc32fast 1.5.3; serde 1.0.229.
- [scancraft-usb](../../crates/scancraft-usb/Cargo.toml): nusb 0.2.9; serde_json 1.0.154.
- [scancraft-workflow](../../crates/scancraft-workflow/Cargo.toml): serde 1.0.229; serde_json 1.0.154; crc32fast 1.5.3; libc 0.2.191; image 0.25.10.

Core reuse is already substantial: nalgebra supplies matrices/SVD/eigensolvers;
serde/serde_json supply serialization; crc32fast supplies CRC; image supplies
PNG/JPEG codecs; eframe/egui/glow/winit provide desktop/window/GPU presentation;
libc and v4l2-sys-mit provide Linux ABI access. nusb belongs to the optional USB
experiment, not the GUI acquisition backend. Exact selected features are in each
manifest; app explicitly selects glow, Wayland, X11 and fonts, not eframe's wgpu
backend. No OpenCV/Open3D/PCL/CCCoreLib/VCGlib production dependency is declared.
Python OpenCV/NumPy/SciPy comparison tools are separate development oracles,
not Rust scanner runtime dependencies (tools/*requirements*.txt and compare_*).

## Native build/runtime dependencies

README.md requires Linux V4L2 development headers, build-time libclang for UAPI
bindings, and desktop graphics libraries. These system packages/driver versions
are **not pinned by Cargo.lock**. Exact distro packages must be recorded in future
reproducible binary packaging. GPU OpenGL/GLX/EGL, X11/Wayland/window integration
comes through the selected GUI stack. The acquisition workstation's existing
UVC/Y14/metadata patch and device permissions remain an installation constraint;
no stock-kernel portability qualification exists. The optional kernel experiment
is separately licensed and is not silently installed by the Rust build.

`rust-toolchain.toml` pins Rust 1.99.1 with rustfmt/clippy. `tools/check.sh` uses
--frozen for Clippy/tests/release; missing dependencies fail rather than download.
The GitHub workflow uses its recorded locked Cargo steps; do not claim its debug
CI is a release benchmark. Setup/fetch is an explicit online preparation step
(DEVELOPMENT_REVIEW.md, .github/workflows/ci.yml).

## Current maintenance and proposed upstream tracking

**Confirmed:** Cargo manifests/lock and checked-in notices manage dependencies.
No Dependabot/Renovate configuration or automatic algorithm-porting daemon was
found in the tracked .github tree. CodeRabbit reviews code changes, not a proof of
numeric equivalence or an automatic dependency-license updater. Existing reference
scripts are invoked explicitly; they do not monitor upstream algorithm commits.

**Recommendation:** monitor crate advisories/releases and upstream modules through
read-only scheduled reports or a manually run tool, then propose bounded PRs.
Track only actual dependencies and intentionally chosen algorithm references.
Map an upstream fix to a named local method before proposing a port; similar
algorithm names do not mean equivalent code, layouts, stopping criteria or bugs.
An update proposal should retain upstream URL/revision, license/notice diff,
applicable local files, counterexample/expected behavior, numerical regressions,
benchmark dataset/machine and comparison limits. Run differential and adversarial
cases, preserve an isolated per-checkout target directory, and require reviewer
and owner approval. Do not automatically commit/merge mathematical changes.

Feasibility: crate version/security monitoring is straightforward; semantic
comparison of independently written Rust against C++ is research-intensive and
cannot safely be reduced to patch copying. Costs include C++/Python oracle builds,
fixtures, licensing review, API/version drift, undefined-behavior differences,
noise/degeneracy coverage and real-hardware trials. Algorithm papers may not
supply executable upstreams. A monitor can find candidates; it cannot certify
physical accuracy, safe scanner controls or correct downstream adaptation.
This is a proposed system, not implemented audit work.

## Complete locked external inventory

License declarations/revisions/notices are copied as factual metadata from the
existing publication inventory. Preserve the underlying license texts and any
NOTICE/font terms when actually redistributing the selected code/binaries.

| Package | Locked version | Declared license | Reported upstream revision | Retained notice files |
|---|---|---|---|---:|
| ab_glyph | 0.2.32 | Apache-2.0 | ffde4c677fa1095ac355174c1aa2919d7d53eff3 | 1 |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 | 971e5a9b337fbae3edb1ac3ac44d84351d24e8b8 | 1 |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | 89a031a0f42eeff31c70dc598b398cbf31f1680f | 3 |
| ahash | 0.8.12 | MIT OR Apache-2.0 | 9aa1ba20f05ed582eda04ea625d5658c92195a57 | 2 |
| aho-corasick | 1.1.5 | Unlicense OR MIT | 5178060ce73d91938f8582d0360e3be031380440 | 2 |
| android-activity | 0.6.1 | MIT OR Apache-2.0 | b4ddf059b77be12cdb955d394e65a92e7568d936 | 3 |
| android-properties | 0.2.2 | MIT | 35f123fa53be399307eb861b033a62f0ed193d83 | 1 |
| approx | 0.5.1 | Apache-2.0 | 13fcbc78dcda4d0136de07c36a7979d545f08e38 | 1 |
| arboard | 3.7.0 | MIT OR Apache-2.0 | 5f5c8637911ba219edee619105a19b4f74e66a9f | 2 |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | 0cb664cf505844348538230479b0040b44f3faf1 | 2 |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 | 0f7d9bf88fc3e68a66c4e2f5130cddbdd2ae43f0 | 2 |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | 9969a29ccd1fe298e2df7346849c0095ba303a64 | 3 |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | 2799b09c24e6632f8e653c5cd8fc303e85a906ba | 2 |
| bindgen | 0.65.1 | BSD-3-Clause | 7d243056d335fdc4537f7bca73c06d01aae24ddc | 1 |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | 2b8861d23b53d63d326d2694ddabde6e7b19fbd0 | 2 |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | 16d8d5ed27e9d379ae5d531ffcf7535cd02ef8f3 | 2 |
| bitflags | 1.3.2 | MIT/Apache-2.0 | ed185cfb1c447c1b4bd6ac021c9ec3bb02c9e2f2 | 2 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | 80ce9b545acb0bd42150695fc351889cac1d8eb4 | 2 |
| block2 | 0.5.1 | MIT | 4fc083f1c6d6784577e38b0ee8dbd344481e2fd2 | 1 |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | 84654ace6be4444da3ff102a0a0af3b38c4df4fb | 2 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | f363643e951a7ac9e4b9921de982f2b0918902e3 | 3 |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT | 839ee557dd9b1ac5a0b0155cabba71f96b4ccd06 | 3 |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | 7d44da06a48fe10d7e23efd9a932d18a7a2def63 | 2 |
| bytes | 1.12.1 | MIT | 76c0fbb54ed4336caf9d2311658a2f4a5627c21d | 1 |
| calloop | 0.13.0 | MIT | 72ab703e65681001aa69df798cf9fcd729b78a74 | 1 |
| calloop | 0.14.5 | MIT | c0d9940eb4764e6a244f0ee46bc99e988f9d9b7c | 1 |
| calloop-wayland-source | 0.3.0 | MIT | 47ce61c2629c102bb37e893640037610e29dfd38 | 1 |
| calloop-wayland-source | 0.4.1 | MIT | 9828ce382a6b3a2b4b688048314939f7c517bcc3 | 1 |
| cc | 1.10.0 | MIT OR Apache-2.0 | 8d06fcb73f6938c35c9bc59edbdcb9fb56f04d71 | 2 |
| cexpr | 0.6.0 | Apache-2.0/MIT | c7ccdfbc37b508cfda1171ab4f89afaeb72e82f3 | 2 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | 8b0a2b0fc1fa3b3741b9e72895f390cf499fe36f | 2 |
| cfg_aliases | 0.2.2 | MIT | e069ce61e00fee423ac1dbe6a897f228f1774716 | 2 |
| cgl | 0.3.2 | MIT / Apache-2.0 | 46348b6c10543dd6465d47a7e14ee0f37516e08e | 2 |
| clang-sys | 1.9.1 | Apache-2.0 | 8844c21292e54818d200980cd90a5498e2a5a644 | 1 |
| clipboard-win | 5.4.3 | BSL-1.0 | ed27008ae5d1f10cfbb266de039f0fb35fd05a56 | 1 |
| codespan-reporting | 0.12.0 | Apache-2.0 | 3d3a03a8fcc30cc10b783b16c62029d5c232059b | 1 |
| combine | 4.6.9 | MIT | a2da738ca628bb2e2ef9881fce8e35cbafc455b2 | 1 |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT | e874f701f8413de01948c4903f894e5c845d8950 | 2 |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | 548b65cd3046bb46fc5cebb9f39e7bfd56eaeaa2 | 2 |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 | 795398aa3775bbdaf03419fcd2cc0dad7ebbc4dc | 2 |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | 652bab0a62d87df8a974ffad2ad9f7be218a8b56 | 2 |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 | 07be553c3f1d435b5087c16e787d73cb24b65efb | 3 |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 | d4ce710182f1756c9d874ab917283fe1a1b7a011 | 2 |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 | d2363eab95ebc85fcf726d1c8fa845148e474591 | 2 |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | 2920c984290229ab4e0ca0452ef09e48a82063f3 | 2 |
| crunchy | 0.2.4 | MIT | ba7b86cea6ba89ccfc72ccb24cc4a4ac6d9c6272 | 1 |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib | 766287581de8b4895d018c2d15ada77627a954f0 | 3 |
| dispatch | 0.2.0 | MIT | 82d6c7a5b75dc0c71c3f46f87bb6c16a476f7748 | 0 |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | 8852b424193ca41602281b3d7540d7c8ed51e49a | 1 |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | 20016487354a96b22a7b97974483ae4f51a1bddc | 2 |
| dlib | 0.5.3 | MIT | ab1a79fd8b92a331a5f123be9e1dcbbcbd48dbf8 | 1 |
| document-features | 0.2.12 | MIT OR Apache-2.0 | 721e708012254b81760bd2befdae0970e7a2615e | 2 |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 | c4c48bad50482df2544115ba971acfd5916e6140 | 2 |
| dpi | 0.1.2 | Apache-2.0 AND MIT | 587ade844dfb0eada3696ba1cb263c66eea80581 | 2 |
| ecolor | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| eframe | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| egui | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| egui-wgpu | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| egui-winit | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| egui_glow | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| either | 1.19.0 | MIT OR Apache-2.0 | 1e4326df5972eb4a0ced66cb3b67fa0c5aca8493 | 2 |
| emath | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| epaint | 0.33.3 | MIT OR Apache-2.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 2 |
| epaint_default_fonts | 0.33.3 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | 44cdd653e2317d300fb8a6c9c36b03f23991e803 | 4 |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | 4ee88130bd4ab3229e785533da2af641be22deac | 2 |
| errno | 0.3.14 | MIT OR Apache-2.0 | ffc03bfb9eb491013567115e6eea560948cd9e52 | 2 |
| error-code | 3.4.0 | BSL-1.0 | e4615e514db3ff64f5f1b328b4865b20a3dcdbc3 | 1 |
| fax | 0.2.7 | MIT | ac2a4b2237d98830ea1fd37c7d280eec0e87c1d8 | 1 |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | c365c7e6ffa81feb2e1fb762eed7299f05c9b0ca | 2 |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | ffbb10da608f715ed69a4561c4cbdca7c6fbbba6 | 2 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | ed93d4fc60eaf876c6aded741bf992d524551930 | 2 |
| foldhash | 0.2.0 | Zlib | 8f878c636fda9c9e93384824ea45e06d03f009f5 | 1 |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | c5a90ed3f39dd5fe8b5af6e9cee1bc446746272c | 2 |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 | 2d5077646c441fb089662e9ac439bec45c0d84b5 | 2 |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | bdce800c23ce73b33a337833611e0e5fdf83ce9f | 2 |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | 91377f48bf35011d042aa5abef9e7f2a0a625aaa | 2 |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | 705e6b5c0f06535b1aac1cb1989a172b3d45be8c | 2 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | 705e6b5c0f06535b1aac1cb1989a172b3d45be8c | 2 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | 705e6b5c0f06535b1aac1cb1989a172b3d45be8c | 2 |
| gethostname | 1.1.0 | Apache-2.0 | 7cffc339711704f29f72d51ebc483b3f28f10099 | 1 |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | 38e4ad38309a85b56eef4fc759535ccfc322ba9a | 2 |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | 5e7cd5733536844a9856dc7259bd4696bbe5e3ae | 2 |
| gl_generator | 0.14.0 | Apache-2.0 | ea503e8d5fb6d73c6030e6191ce738cd3bf3433e | 1 |
| glam | 0.30.10 | MIT OR Apache-2.0 | 990737c58c8c6f4315a4ad09806aa55aa141a503 | 2 |
| glam | 0.31.1 | MIT OR Apache-2.0 | 2f54355b7ecb4670685f103a9742747784af90e4 | 2 |
| glam | 0.32.1 | MIT OR Apache-2.0 | e8ed32dcd0232b8078b2be91d6fd8f3b99c28842 | 2 |
| glam | 0.33.12 | MIT OR Apache-2.0 | e7d680ed34c051cfa8741f76821628a925161280 | 2 |
| glob | 0.3.4 | MIT OR Apache-2.0 | cfa2a58f2e44373573f657ec25b3621e44714dee | 2 |
| glow | 0.16.0 | MIT OR Apache-2.0 OR Zlib | 83c7b9b59ec4b26942801b3d0164db6d396a8c74 | 3 |
| glutin | 0.32.3 | Apache-2.0 | 20d1c103172aa4025f02cc94ca16a3169bea789c | 1 |
| glutin-winit | 0.5.0 | MIT | 0811b6c753334f1681c7db8a48e407596cfa9366 | 1 |
| glutin_egl_sys | 0.7.1 | Apache-2.0 | d9b4fd7036386c5f23b0a1015a31a986805d4db3 | 1 |
| glutin_glx_sys | 0.6.1 | Apache-2.0 | d9b4fd7036386c5f23b0a1015a31a986805d4db3 | 1 |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 | d9b4fd7036386c5f23b0a1015a31a986805d4db3 | 1 |
| half | 2.7.1 | MIT OR Apache-2.0 | 8cc891f3e4aad956eca7fa79b1f42f87ecd141ae | 2 |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | 1876e4f02708b93903d55ef598f68e82a826518f | 2 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | c62a63a61b7caf2de8f9ecb7b06a66b0ab6bdf3d | 2 |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 | c0b97d12a2f1f65610ba84e6f23fbb69fc70c0f7 | 2 |
| hexf-parse | 0.2.1 | CC0-1.0 | 4225763d744183d720f575ae96d04161b4d08ea0 | 0 |
| home | 0.5.12 | MIT OR Apache-2.0 | 4f15cc8882fb34bf70c945e9c8ae91d6c8c91757 | 2 |
| icu_collections | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_locale_core | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_normalizer | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_normalizer_data | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_properties | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_properties_data | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| icu_provider | 2.4.0 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| idna | 1.1.0 | MIT OR Apache-2.0 | 91377f48bf35011d042aa5abef9e7f2a0a625aaa | 2 |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | 1d9782f32c9e29218542b4a9411a82b35ec5c8cb | 2 |
| image | 0.25.10 | MIT OR Apache-2.0 | 76e57184f22772dad1138e96954e57945406b15e | 2 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | 41a870887c4c77adf665886e63df08f406bfe37a | 2 |
| io-kit-sys | 0.5.0 | MIT OR Apache-2.0 | 1c969a9d17b9d98ae7fe2d3f958c7547e4380a74 | 2 |
| itoa | 1.0.18 | MIT OR Apache-2.0 | af77385d0daf4d0e949e81f2588be2e44f69f086 | 2 |
| jni | 0.22.4 | MIT OR Apache-2.0 | 5ae9458a4ec44c5318f37ddc7569c1d4ae8a69e7 | 2 |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 | 33045a124105c939d1e2cbdcb5a39e5d868ffa03 | 2 |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 | 1985e8bdd087b768032e03bbc8b6cd574a574ffd | 2 |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | 65ea279041232aec345dbd9483b3956c788862aa | 2 |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | 64d77b7a5f119d7b55b4e2c169a4668067ff59e6 | 2 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 | 85ba04e3cf89e5c10e7b82de69f6be6dff9d3ec3 | 2 |
| js-sys | 0.3.106 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| khronos_api | 3.1.0 | Apache-2.0 | f150967b1c44ae888e6676f93f639ebc82771bdc | 1 |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 | 4c1b9a170c157d679592e2682b13cc780c1db814 | 2 |
| lazycell | 1.3.0 | MIT/Apache-2.0 | b78f3f10c0d005b81d97011fd7f98fc3cdff5982 | 2 |
| libc | 0.2.191 | MIT OR Apache-2.0 | ccc2ff4aaab962f7fde7b8edda7379c045b1ca12 | 2 |
| libloading | 0.8.9 | ISC | f4ec9e702de2d0778bccff8525dc44e4cacac2d1 | 1 |
| libm | 0.2.16 | MIT | dfd2203a4d6110820ad7bb65cafe1bf331a03a3d | 1 |
| libredox | 0.1.25 | MIT | 9a9fc664b027a93a2fa5b27167858956c7566d2d | 1 |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 0e2918cf3e366d9c923d4ca05f169b49d826db56 | 4 |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 77e4a4223b14957ad8fa2958ea411cd4b54314e7 | 4 |
| litemap | 0.8.3 | Unicode-3.0 | a14f2dad852be26bad277ae704abf27a15cbfed1 | 1 |
| litrs | 1.0.1 | MIT OR Apache-2.0 | 510d5d778413d51da7d11f8e498d27bba86739e1 | 2 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | d7828fff7b5d6327ae608e82db45f888b344449a | 2 |
| log | 0.4.34 | MIT OR Apache-2.0 | 8034743dd9d7f7583bd9a670271483d176130911 | 2 |
| mach2 | 0.5.0 | BSD-2-Clause OR MIT OR Apache-2.0 | d623c364252ce4dd9d7731bf0ca2d842c5252d1e | 3 |
| matrixmultiply | 0.3.11 | MIT/Apache-2.0 | 07f968c20e0df41e3fd694e9c47ef5368ec9eee1 | 2 |
| memchr | 2.8.3 | Unlicense OR MIT | 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d | 2 |
| memmap2 | 0.9.12 | MIT OR Apache-2.0 | 4692b056b327aa08dace3a7e345a99e4cff77534 | 2 |
| memoffset | 0.9.1 | MIT | 153fc3d50f03755be53148bc3eb97390f9011e45 | 1 |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | e997c46656ebe83e696b866bd954da1fa3f64eef | 3 |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | 44e43c7786e379b2b1a7fde4aa0e63be719e583d | 4 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | 4e582392df3a739d2b0dfd2c537dc33e8942be38 | 4 |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | c4affa196727d17f21a7f6f148a558867decb470 | 2 |
| naga | 27.0.3 | MIT OR Apache-2.0 | d61dc3af8856f0cd9023c5e888b791d1b7118559 | 2 |
| nalgebra | 0.35.0 | Apache-2.0 | 5f927f6c821d0ced07a0e086e5f37251cab2a8c3 | 1 |
| ndk | 0.9.0 | MIT OR Apache-2.0 | 49bbbba16c58ff63cb8a0ad0eca5a9fb7ecaec25 | 2 |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 | 10f2ba388fca20f7349996ebae26ccda7a6fda5c | 2 |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 | 49bbbba16c58ff63cb8a0ad0eca5a9fb7ecaec25 | 2 |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT | 99b021664e9d7fd3e1f7ffb8633a137e21e12149 | 2 |
| nom | 7.1.3 | MIT | 869f8972a4383b13cf89574fda28cb7dbfd56517 | 1 |
| num-bigint | 0.4.8 | MIT OR Apache-2.0 | 65330056d6c00def4c6cda78c87453f4c2a76913 | 2 |
| num-complex | 0.4.6 | MIT OR Apache-2.0 | 91fdc06356c0c868cb88b5a180859023c57e6e50 | 2 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 | 765fd9b3eb9397471d41b9f613e530f6d9d6383f | 2 |
| num-rational | 0.4.2 | MIT OR Apache-2.0 | 4d55ad22ac86ebbc4cb45d79a956e4a1f7af57d1 | 2 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | 7ec3d41d39b28190ec1d42db38021107b3951f3a | 2 |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | f11d81c6cb644489f8a0fc24f9eea9a53d66f92e | 3 |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | f11d81c6cb644489f8a0fc24f9eea9a53d66f92e | 3 |
| nusb | 0.2.9 | Apache-2.0 OR MIT | 5dfee4c1f82a1fdf019c3b5935ee4cd887908093 | 2 |
| objc-sys | 0.3.5 | MIT | 4fc083f1c6d6784577e38b0ee8dbd344481e2fd2 | 1 |
| objc2 | 0.5.3 | MIT | 5237cbf081d04ef75a1f6a207b4018e7c1ae7438 | 1 |
| objc2 | 0.6.5 | MIT | d7d2fa23ceaa5e6096c923b081040e5d81b3b9df | 1 |
| objc2-app-kit | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 7b1abfd750a2cacaea71d6a56ecfb83cb7de560b | 1 |
| objc2-cloud-kit | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-contacts | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-core-data | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 7b1abfd750a2cacaea71d6a56ecfb83cb7de560b | 1 |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 7b1abfd750a2cacaea71d6a56ecfb83cb7de560b | 1 |
| objc2-core-image | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-core-location | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-encode | 4.1.0 | MIT | 8d214f5477365ffcbcbb7de058c86ed9a518efb7 | 1 |
| objc2-foundation | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-foundation | 0.3.2 | MIT | 7b1abfd750a2cacaea71d6a56ecfb83cb7de560b | 1 |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 7b1abfd750a2cacaea71d6a56ecfb83cb7de560b | 1 |
| objc2-link-presentation | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-metal | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-quartz-core | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-symbols | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-ui-kit | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-uniform-type-identifiers | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| objc2-user-notifications | 0.2.2 | MIT | e282618be4c3a3b9542957e0c8540e9588472ce8 | 1 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | 80fe900b21f6d76c1a2ed74d3343e8a3a88c46d0 | 2 |
| orbclient | 0.3.55 | MIT | 5e16bc813e6c53fe40a6145975b2566fb0e890b6 | 2 |
| owned_ttf_parser | 0.25.1 | Apache-2.0 | 3abee95150b4c32f16ffbaf1bb922a68b465d48b | 1 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | d7828fff7b5d6327ae608e82db45f888b344449a | 2 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | d7828fff7b5d6327ae608e82db45f888b344449a | 2 |
| peeking_take_while | 0.1.2 | Apache-2.0/MIT | not recorded | 2 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | 91377f48bf35011d042aa5abef9e7f2a0a625aaa | 2 |
| pin-project | 1.1.13 | Apache-2.0 OR MIT | c3b6b85f7e1674602c0ed693da985b33b2452c3c | 2 |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT | c3b6b85f7e1674602c0ed693da985b33b2452c3c | 2 |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | 3bdf763446aa78f90e3bdac1ef583e014832ab4c | 2 |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 | a8852cc148f8f7d504bf3a62444c298dcd3637f0 | 2 |
| plain | 0.2.3 | MIT/Apache-2.0 | not recorded | 2 |
| png | 0.18.1 | MIT OR Apache-2.0 | 2a3f980245e3ae38b82ade96533e7b450e8477bb | 2 |
| polling | 3.11.0 | Apache-2.0 OR MIT | f404d08dd2a61994ac5fe0ed663bc051bd94c053 | 2 |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | e4595528211f1d53b7f4d93e528ef84b491fc871 | 2 |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT | 6fb276aeaee508914ef33918f2e3f003d6808ada | 2 |
| potential_utf | 0.1.6 | Unicode-3.0 | a14f2dad852be26bad277ae704abf27a15cbfed1 | 1 |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 | c971184fa8c5ef5a2828196e35bd99469455b46b | 2 |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 | 8f3d8b04539f732e09c07907f138a248c8d0eed9 | 2 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | ed8a5497669cd63db33bf24646f261b012bbbc4a | 2 |
| profiling | 1.0.18 | MIT OR Apache-2.0 | 8271551172eb6fa4cba47369aedd93790c623df9 | 2 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | acf7acd00cfc845307eda7b7b2de19e42303de82 | 2 |
| quick-error | 2.0.1 | MIT/Apache-2.0 | 25ab982bb0c43130ae48485151f4a232eeab7d0c | 2 |
| quick-xml | 0.41.0 | MIT | 4deda08abeffdc188c269360229cf47e12a77a9f | 1 |
| quote | 1.0.47 | MIT OR Apache-2.0 | 723dcb47d3f0ddc896e17287c8a8d3f2ea2317d5 | 2 |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 97b55bed1c2c91dcbf787674849f05337ff80b33 | 0 |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 7e1b0322d31d625f81a5656096330934f9cd835d | 0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | 5fda8e8420b069368e9450e70c2869e32dcdffc1 | 3 |
| rawpointer | 0.2.1 | MIT/Apache-2.0 | e4f5800e86acc8cb610430a3f361977480e2b0a6 | 2 |
| redox_syscall | 0.4.1 | MIT | f94a5a7dcafdcbcd8d43af586a637148515bc86a | 1 |
| redox_syscall | 0.5.18 | MIT | 15aa8e32ef0f3328dd9d48cc8ca02e148a362069 | 1 |
| redox_syscall | 0.9.5 | MIT | 716a8f83f452054b8adc8bf568f7bd35d46a5f31 | 1 |
| regex | 1.13.1 | MIT OR Apache-2.0 | 2b527599eb9eea0dcc288c704584f242f26a5c61 | 2 |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | e6ad938198e68339a593f85d228f2d44cb54718f | 2 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | 140167995737fa11dfe11b8af8b9aa143b790b4e | 3 |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 | 0276afc3686dd3ee9475798ed8ab89026d74143d | 2 |
| rustc-hash | 1.1.0 | Apache-2.0/MIT | 6fc9a706a9312ee801c7e77ca06c0782fa9eadef | 2 |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | c13e7ccca705e6255387a2ebc6dca142d6881621 | 2 |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | eeca449cca83e24150e46739e797aa82e9142809 | 2 |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | acf4a284ea893efa93953e1ff2f2f0ab00be1cdc | 4 |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 287214b889865d8e1406a0ee71cc409b6f6191c8 | 4 |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | 3a7c76605450b9a7299c6502a421909de9126a59 | 2 |
| safe_arch | 1.2.0 | Zlib OR Apache-2.0 OR MIT | d45ee8c53c38fdddee2eade0a5ddef1a3c677b4c | 3 |
| same-file | 1.0.6 | Unlicense/MIT | 5799cd323b8eefd17a089c950dac113f66c89c9e | 2 |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | c0ff7bf6d33e568353ed863d90f893e7e80a0ed1 | 2 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | bb988222848d39b0b9037b29da3a3a1eb05ebf4b | 2 |
| semver | 1.0.28 | MIT OR Apache-2.0 | 7625c7aa3f0e8ba21e099d1765bcebcb72aa8816 | 2 |
| serde | 1.0.229 | MIT OR Apache-2.0 | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 | 2 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 | 2 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 | 2 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8 | 2 |
| shlex | 1.3.0 | MIT OR Apache-2.0 | 4a0724b0b62ef715467875b040a890ce75a8a829 | 2 |
| shlex | 2.0.1 | MIT OR Apache-2.0 | e82b1411beb7c92871c2c078c9ab415bbcf207ef | 2 |
| simba | 0.10.2 | Apache-2.0 | 16a77ffd653f5842028c53681460e20386428966 | 1 |
| simd-adler32 | 0.3.10 | MIT | d93164f98d7fe2daf883f2e1bec82778dd26199a | 1 |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT | b7977cfbebc962b6fa68969bac9c863ae55a8233 | 2 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | 87ee8d9d20b849eae1974a82b248ee6fe7491bcc | 2 |
| slab | 0.4.12 | MIT | a1e4346070a48c936d808de75191dee5d01e433c | 1 |
| slotmap | 1.1.1 | Zlib | 2d56154b9b628f7d7a506f24ebc4f55a13ec8fed | 1 |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | eab77462eda2767d2386f1d251928f474827e1c7 | 2 |
| smithay-client-toolkit | 0.19.2 | MIT | c583de8dd5651f8168c6513cd282137c42aae049 | 1 |
| smithay-client-toolkit | 0.20.0 | MIT | 8a0b33616145b38c7f989f8e468d57e69a2a6748 | 1 |
| smithay-clipboard | 0.7.3 | MIT | 26c2f53f15f6bdc4f41a442d0ae2c2d63bbc617c | 1 |
| smol_str | 0.2.2 | MIT OR Apache-2.0 | aea39b4974f55e9380556272b1335e2f6fd13cc7 | 2 |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | 30002b4228f7cdee309217b6bf6eee099dda0b00 | 2 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | 18bc65a094d890fe1faa5d3ccb70f12b89eabf56 | 2 |
| syn | 2.0.119 | MIT OR Apache-2.0 | 3295f9e9841785ac88a5e558c884854d5fb7d67f | 2 |
| syn | 3.0.9 | MIT OR Apache-2.0 | 7c44a5565b4421035101cc16184e9af08ab5fc77 | 2 |
| synstructure | 0.14.0 | MIT | 30cfafe36e42405e717a8e55da042a2835a698d7 | 1 |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | 41938bd3a03a70d34ed8e53d99c89c770c7c9c41 | 2 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | b1827ee06f81a7d7f954e676771e19a97f9f8e3b | 2 |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | 41938bd3a03a70d34ed8e53d99c89c770c7c9c41 | 2 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | b1827ee06f81a7d7f954e676771e19a97f9f8e3b | 2 |
| tiff | 0.11.3 | MIT | e41159ce058bf193718b017599e2d7d53e9ccc92 | 2 |
| tinystr | 0.8.5 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| tokio | 1.54.1 | MIT | b53b2001ed2ccfd9b87e32a0c6da1a1eebdc80b8 | 1 |
| toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | f72038c2508112dfaaedb227a511f91bf9b2015c | 2 |
| toml_edit | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | ceac6e4412420d5a5c19e4f7293ad005cf1a31b2 | 2 |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | ceac6e4412420d5a5c19e4f7293ad005cf1a31b2 | 2 |
| tracing | 0.1.44 | MIT | 2d55f6faf9be83e7e4634129fb96813241aac2b8 | 1 |
| tracing-core | 0.1.36 | MIT | 10a9e838a35e6ded79d66af246be2ee05417136d | 2 |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 | 56c33b910b03ca152f78363ec471c5dfd97c3069 | 3 |
| type-map | 0.5.1 | MIT/Apache-2.0 | d3ab3e642736d87f3fa8b1e6c8f13a93fcfb72e1 | 2 |
| typenum | 1.20.1 | MIT OR Apache-2.0 | 0db9a0f731981f29266b63586c29fa07e4477b1a | 3 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | c1ac3afb721f3cea6527bc2255340a04d0de3ca0 | 3 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | 66a032fd8d667bc47ac5b640b151dff3f5356d07 | 3 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 | 9d98411769fe13c7c18cab0b3fbbab29ba8350ea | 3 |
| url | 2.5.8 | MIT OR Apache-2.0 | d6ea13c5f8e7e6e627f6390161b3e185bda5e5ce | 2 |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | 6b604ea2365bd535fb66eb4bfe92a9e00e333090 | 3 |
| v4l2-sys-mit | 0.3.0 | MIT | cf6d4125d3f37f24d18c325fd369f503d9f12a88 | 1 |
| version_check | 0.9.5 | MIT/Apache-2.0 | d77ef9f27cc336719b2d839d09ee6635dd22f758 | 2 |
| walkdir | 2.5.0 | Unlicense/MIT | 4f26be4d450910916ea11533b2efc52b9a6483bc | 2 |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 1edbc7add705b075d28436b26057ca9aa5a5a492 | 3 |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| wayland-backend | 0.3.17 | MIT | 72f7fe0d9e99cb720ed57ade0c3077aec533b6ab | 1 |
| wayland-client | 0.31.15 | MIT | 05196740da57e41b60a9e9f8e35079e9bd29b89a | 1 |
| wayland-csd-frame | 0.3.0 | MIT | e9a510b13a96e9a1fabb31d3bf3f28cab702d003 | 1 |
| wayland-cursor | 0.31.14 | MIT | a3d7927d87799b2955bf491b51c7c2a3a82da661 | 1 |
| wayland-protocols | 0.32.13 | MIT | 661cf141f5de4ec83559125b3d8bb1c79f25116b | 1 |
| wayland-protocols-experimental | 20250721.0.1 | MIT | 3a45d67679d763cb813062530d50d0538f4312ae | 1 |
| wayland-protocols-misc | 0.3.12 | MIT | a3d7927d87799b2955bf491b51c7c2a3a82da661 | 1 |
| wayland-protocols-plasma | 0.3.12 | MIT | a3d7927d87799b2955bf491b51c7c2a3a82da661 | 1 |
| wayland-protocols-wlr | 0.3.12 | MIT | a3d7927d87799b2955bf491b51c7c2a3a82da661 | 1 |
| wayland-scanner | 0.31.11 | MIT | 05196740da57e41b60a9e9f8e35079e9bd29b89a | 1 |
| wayland-sys | 0.31.11 | MIT | a3d7927d87799b2955bf491b51c7c2a3a82da661 | 1 |
| web-sys | 0.3.106 | MIT OR Apache-2.0 | 165586f854c852bcbc4d14d8a75a0a0e0d0abc12 | 2 |
| web-time | 1.1.0 | MIT OR Apache-2.0 | 2d200ded30bbcab66f6a4bfa277ac546685728db | 2 |
| webbrowser | 1.2.5 | MIT OR Apache-2.0 | 278ec1b5aeeb07f9fcbda569cb075e58727ba623 | 2 |
| weezl | 0.1.12 | MIT OR Apache-2.0 | c75ea1f6d935cc56bad8674396b7cb0dd7275f99 | 2 |
| wgpu | 27.0.1 | MIT OR Apache-2.0 | c76dea031c688ecef0050dbf60506fff128fb23f | 2 |
| wgpu-core | 27.0.3 | MIT OR Apache-2.0 | d61dc3af8856f0cd9023c5e888b791d1b7118559 | 2 |
| wgpu-core-deps-windows-linux-android | 27.0.0 | MIT OR Apache-2.0 | 482a983e1006333dc249bd1a08b04db3e4a48230 | 2 |
| wgpu-hal | 27.0.4 | MIT OR Apache-2.0 | af91efa93da2af9366b252ed7b9a089175f4874e | 2 |
| wgpu-types | 27.0.1 | MIT OR Apache-2.0 | c76dea031c688ecef0050dbf60506fff128fb23f | 2 |
| which | 4.4.2 | MIT | 35196c186f8fbec8dc40919cffa726a8bfc10998 | 1 |
| wide | 1.7.1 | Zlib OR Apache-2.0 OR MIT | a0519073776eb89579b8e12c6c7689df13f93c53 | 3 |
| winapi-util | 0.1.11 | Unlicense OR MIT | 803874c57dc1f10ecd42f7c86d9de72f53818432 | 2 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | d468916ac27a36fb8a12bafc1bf5c0ec2fe92238 | 2 |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | 3a605cba064b26f2a198ac58085f8c8836f47c38 | 2 |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | 308e08ec259027ebbef11b8ef838923626bf821e | 2 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | 32c3144490c016fe496a0aed769bce60987a2e9d | 2 |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | db06b51c2ebb743efb544d40e3064efa49f28d38 | 2 |
| winit | 0.30.13 | Apache-2.0 | e9809ef54b18499bb4f2cac945719ecc2a61061b | 1 |
| winnow | 1.0.4 | MIT | 7539ec0fc27144bfdcf9a68b0dbbec48bd0d5bae | 1 |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 2e00369a643c0c8048b8636401e36b0cbf2dfb05 | 3 |
| writeable | 0.6.4 | Unicode-3.0 | a14f2dad852be26bad277ae704abf27a15cbfed1 | 1 |
| x11-dl | 2.21.0 | MIT | fced94ef6eb5935c892079a46812806f7b7a9237 | 1 |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | 1b73245acc5dce4c4987c80dd7589e7b60af3532 | 2 |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 | 1b73245acc5dce4c4987c80dd7589e7b60af3532 | 2 |
| xcursor | 0.3.11 | MIT | 4760fa50b9352168e1b740c1ff75f37697620423 | 1 |
| xkbcommon-dl | 0.4.2 | MIT | 631fcaac6bb5607585527d8e89ddfc26fa8ec285 | 1 |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib | 77596eb8319f8d7a97db6477f307a94a0905c439 | 3 |
| xml-rs | 0.8.29 | MIT | 33cdf417b112c0ebd1f0858b2a67464b3712a4c6 | 1 |
| yoke | 0.8.4 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| yoke-derive | 0.8.4 | Unicode-3.0 | a59ab860d4bda548e94dfbf992d87fe1f761bc55 | 1 |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | c55af491426ef53f35fb8c6e83f220fa07712e66 | 3 |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | c55af491426ef53f35fb8c6e83f220fa07712e66 | 3 |
| zerofrom | 0.1.8 | Unicode-3.0 | e9575f7e4b5aeddfeeb87ada5d3e715950e8233e | 1 |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | ce235d80cf7cbd028e8bd59920420e1203b205b3 | 1 |
| zerotrie | 0.2.5 | Unicode-3.0 | a14f2dad852be26bad277ae704abf27a15cbfed1 | 1 |
| zerovec | 0.11.9 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| zerovec-derive | 0.11.7 | Unicode-3.0 | 031d9e89f6a3fd26a14491e090729d46094328fd | 1 |
| zlib-rs | 0.6.8 | Zlib | 0254e07204884bfc902d6ebb68c8b703d0ea49cf | 1 |
| zmij | 1.0.23 | MIT | 7b7cc48b58028e8af7be87e94c0c1c8936f1a57c | 1 |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib | de5dfc800cfa063116eb695edb6ccd01cb96ea7a | 3 |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | 31d81fed7551c8ccea456d9d8e2b1fd8bebb6995 | 3 |
