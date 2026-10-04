#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if black_vault_desktop_lib::run().is_err() {
        // Startup diagnostics must not expose storage paths or internal errors.
        eprintln!(
            "Black Vault could not start. Check local storage permissions and restart the application."
        );
        std::process::exit(1);
    }
}
