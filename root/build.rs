use ::std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    const LINK: &str = "root/layout.ld";
    const SELF: &str = "root/build.rs";

    println!("cargo:rerun-if-changed={}", LINK);
    println!("cargo:rerun-if-changed={}", SELF);

    println!("cargo:rustc-link-arg=-T{}", LINK);
    Ok(())
}
