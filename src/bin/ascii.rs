//! Thin wrapper that runs the ASCII terminal renderer directly.
//!
//! The same renderer is available via `wl-harmonograph --ascii` or by running
//! without a Wayland display. This binary exists for convenience.

fn main() {
    wl_harmonograph::ascii::run();
}
