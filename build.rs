fn main() {
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VERSION");
    println!("cargo:rerun-if-env-changed=LUDUSAVI_VARIANT");

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/windows/manifest.rc");
        println!("cargo:rerun-if-changed=assets/windows/manifest.xml");

        embed_resource::compile("assets/windows/manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
