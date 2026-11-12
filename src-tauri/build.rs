use std::path::PathBuf;

fn main() {
    let conf = std::fs::read_to_string("tauri.conf.json").expect("read tauri.conf.json");
    let parsed: serde_json::Value = serde_json::from_str(&conf).expect("parse tauri.conf.json");
    let version = parsed["version"]
        .as_str()
        .expect("tauri.conf.json must have a \"version\" string field");
    println!("cargo:rustc-env=TAURI_CONF_VERSION={version}");
    println!("cargo:rerun-if-changed=tauri.conf.json");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        link_test_manifest();
    }

    tauri_build::build()
}

/// cargo test 的测试二进制不会经过 tauri_build 的 manifest 内嵌流程，
/// 而 tauri 依赖的 comctl32 v6 API（TaskDialogIndirect）没有 v6 manifest
/// 就会在加载时报 STATUS_ENTRYPOINT_NOT_FOUND。这里用 windres 把一份
/// comctl32 v6 manifest 编译成 .res，仅链接进测试目标（不影响正式安装包）。
fn link_test_manifest() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let rc = out_dir.join("floral-test-manifest.rc");
    std::fs::write(
        &rc,
        r#"#include <winuser.h>
1 RT_MANIFEST "floral-test-manifest.xml"
"#,
    )
    .expect("write test manifest rc");
    std::fs::write(
        out_dir.join("floral-test-manifest.xml"),
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="*" name="floral-notepaper.test" type="win32"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#,
    )
    .expect("write test manifest xml");

    // windres 定位：环境变量优先，其次常见 MinGW 安装位置 / PATH
    let windres = std::env::var("WINDRES")
        .ok()
        .map(PathBuf::from)
        .or_else(|| which("windres.exe"))
        .or_else(|| Some(PathBuf::from(r"D:\AI\dev\mingw64\bin\windres.exe")))
        .filter(|p| p.exists());

    let Some(windres) = windres else {
        println!("cargo:warning=windres not found; cargo test binaries may fail with STATUS_ENTRYPOINT_NOT_FOUND");
        return;
    };

    let res = out_dir.join("floral-test-manifest.res");
    let status = std::process::Command::new(&windres)
        .args(["-O", "coff", &rc.to_string_lossy(), &res.to_string_lossy()])
        .current_dir(&out_dir)
        .status()
        .expect("run windres");
    if !status.success() {
        println!("cargo:warning=windres failed; test binaries may fail to load");
        return;
    }

    println!("cargo:rustc-link-arg-tests={}", res.to_string_lossy());
    println!("cargo:rerun-if-changed=build.rs");
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").ok()?;
    path.split(';')
        .map(|dir| PathBuf::from(dir.trim_matches('"')).join(name))
        .find(|p| p.exists())
}
