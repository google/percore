fn main() {
    println!("cargo:rustc-link-arg=-Timage.ld");
    println!("cargo:rustc-link-arg=-Tqemu.ld");
    println!("cargo:rerun-if-changed=qemu.ld");
}
