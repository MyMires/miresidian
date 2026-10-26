use std::path::PathBuf;
use std::process::Command;

#[test]
fn public_c_header_compiles_as_c11() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = std::env::temp_dir().join("miresidian-c-header-smoke.o");
    let status = Command::new("cc")
        .args(["-std=c11", "-Wall", "-Werror", "-I"])
        .arg(manifest.join("abi/api"))
        .arg("-c")
        .arg(manifest.join("tests/c_abi_smoke.c"))
        .arg("-o")
        .arg(&output)
        .status();
    match status {
        Ok(status) if status.success() => {
            let _ = std::fs::remove_file(output);
        }
        Ok(status) => panic!("C ABI header compilation failed: {status}"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("cc unavailable; C header smoke test skipped");
        }
        Err(error) => panic!("unable to run C compiler: {error}"),
    }
}
