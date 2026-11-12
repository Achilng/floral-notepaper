//! 集成测试占位：使 cargo 识别本包存在测试目标，
//! 让 build.rs 的 `cargo:rustc-link-arg-tests`（comctl32 v6 manifest）生效。

#[test]
fn placeholder() {
    assert!(true);
}
