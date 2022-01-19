fn main() {
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VERSION");
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VARIANT");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}
