fn main() {
    slint_build::compile("src/presentation/browser_shell_slint/app_window.slint")
        .expect("failed to compile Slint UI");
}
