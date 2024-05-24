// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

fn main() {
    slint_build::compile("src/ui/main.slint").unwrap();

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
    }
}
