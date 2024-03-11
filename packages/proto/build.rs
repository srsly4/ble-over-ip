use tonic_build;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .compile(&["src/ble.proto"], &["src/"])?;
    Ok(())
}
