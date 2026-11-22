fn main() {
    println!("cargo:rerun-if-env-changed=FOXIDE_RELEASE_VERSION");
    let version = std::env::var("FOXIDE_RELEASE_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned());
    println!("cargo:rustc-env=FOXIDE_RELEASE_VERSION={version}");
    #[cfg(feature = "desktop")]
    slint_build::compile("ui/player.slint").expect("Compile Foxide desktop UI");
}
