#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Force WebView2 to use a fully-transparent background — otherwise the
    // WebView2 control paints an opaque default color OVER the rounded CSS,
    // which makes a visible rectangle in the window's corner areas.
    #[cfg(windows)]
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--default-background-color=00000000",
    );

    windows_island_lib::run();
}
