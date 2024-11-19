fn main() {
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VERSION");
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VARIANT");
    println!("cargo:rerun-if-changed=assets/windows-manifest.xml");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set_manifest_file("assets/windows/manifest.xml");
        res.compile().unwrap();
    }
}
