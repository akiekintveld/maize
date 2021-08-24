use ::std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    const LINK: &str = "kernel/layout.ld";
    const SELF: &str = "kernel/build.rs";

    println!("cargo:rerun-if-changed={}", LINK);
    println!("cargo:rerun-if-changed={}", SELF);

    println!("cargo:rustc-link-arg=-T{}", LINK);
    Ok(())
}
