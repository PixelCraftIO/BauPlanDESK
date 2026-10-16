use std::{env, fs, path::Path};
fn main() {
    let root = Path::new("../core-lib/xbau/2.7").canonicalize().unwrap();
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, files)
            } else if p.extension().is_some_and(|s| s == "xsd") {
                files.push(
                    p.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                );
            }
        }
    }
    walk(&root, &root, &mut files);
    files.sort();
    let source = format!(
        "const SCHEMAS: &[(&str, &[u8])] = &[{}];",
        files
            .iter()
            .map(|n| format!(
                "({n:?},include_bytes!({:?})),",
                root.join(n).to_str().unwrap()
            ))
            .collect::<String>()
    );
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("xbau_schemas.rs"),
        source,
    )
    .unwrap();
}
