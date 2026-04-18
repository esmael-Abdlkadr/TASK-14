//! Minimal native test target so the frontend crate has automated coverage artifacts.
//! (WASM/browser tests are optional; CI can run `cargo test` in `frontend/`.)

#[test]
fn frontend_smoke_spec_placeholder() {
    assert_eq!(2 + 2, 4);
}
