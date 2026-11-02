# Progress

- 2026-10-30: Dependabot bumps fixed on `fix/dependabot-ci` (fax, jpeg-encoder, moxcms, aes/md-5/sha2, x11rb, skrifa, write-fonts, usvg/resvg, hayro-ccitt, patch group); `cargo xtask ci` passes.
- Left out on purpose (API rewrites), now held by ignore rules in `.github/dependabot.yml`: zbus 6 (blocking API removed), harfrust 0.15, vello_cpu/vello_common 0.3; egui and winit stay on the storytold forks.
- 2026-10-31 nightly maintenance: queue empty; added unit tests for `sc-core` pixel containers (`image.rs`: size caps, overflow, bad buffers, bounds, mirror). Tests only.
- Full `cargo xtask ci` hit "no space left on device" late in the run (after all test suites passed); disk-full, not a code failure.
- 2026-11-02 nightly maintenance: queue empty; `cargo xtask ci` showed no failures (cut off by my 50-min timeout during the slow fuzz tests); sc-core tests stable over 3 runs. Added `pdfout.rs` tests (xref offsets, trailer, bad ids, stream length). Tests only.
