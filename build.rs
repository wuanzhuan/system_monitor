fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("src/ui/main.slint", config).unwrap();

    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    #[cfg(windows)]
    {
        println!(
            "{}",
            format!("cargo:rustc-link-arg-bin={pkg_name}=/MANIFEST:EMBED")
        );
        println!(
            "{}",
            format!(
                "cargo:rustc-link-arg-bin={pkg_name}=/MANIFESTUAC:level=\'requireAdministrator\'"
            )
        );
        static_vcruntime::metabuild();
    }
}
