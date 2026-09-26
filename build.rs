fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut resources = winres::WindowsResource::new();
        resources.set_icon("assets/nfo.ico");
        resources
            .compile()
            .expect("failed to embed the Windows application icon");
    }

    let config = slint_build::CompilerConfiguration::new().with_style("native".into());
    slint_build::compile_with_config("ui/app.slint", config).expect("failed to compile Slint UI");
}
