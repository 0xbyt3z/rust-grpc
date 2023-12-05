fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("start compiling the .proto");
    tonic_build::compile_protos("proto/hello.proto")?;
    Ok(())
}