use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let locales_dir = manifest_dir.join("locales");
    println!("cargo:rerun-if-changed={}", locales_dir.display());

    let mut entries = fs::read_dir(&locales_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    let mut generated = String::from("pub const EMBEDDED_LOCALES: &[(&str, &str)] = &[\n");
    for path in entries {
        let code = path.file_stem().unwrap().to_string_lossy();
        generated.push_str(&format!(
            "    ({code:?}, include_str!({path:?})),\n",
            path = path.display().to_string()
        ));
    }
    generated.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("embedded_locales.rs"), generated).unwrap();
}
