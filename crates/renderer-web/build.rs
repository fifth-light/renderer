fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    // 0x3200000 bytes = 32MiB
    println!("cargo:rustc-link-arg=-zstack-size=0x3200000")
}
