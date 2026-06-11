fn main() {
    let conf = std::fs::read_to_string("tauri.conf.json").expect("read tauri.conf.json");
    let parsed: serde_json::Value = serde_json::from_str(&conf).expect("parse tauri.conf.json");
    let version = parsed["version"]
        .as_str()
        .expect("tauri.conf.json must have a \"version\" string field");
    println!("cargo:rustc-env=TAURI_CONF_VERSION={version}");
    println!("cargo:rerun-if-changed=tauri.conf.json");

    tauri_build::build()
}
