fn main() {
    println!("cargo:rustc-link-arg=-Timage.ld");
    println!("cargo:rustc-link-arg=-Texamples/aarch64_qemu/qemu.ld");
    println!("cargo:rerun-if-changed=examples/aarch64_qemu/qemu.ld");
}
