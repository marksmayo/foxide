fn main() {
    #[cfg(feature = "desktop")]
    slint_build::compile("ui/player.slint").expect("Compile Foxide desktop UI");
}
