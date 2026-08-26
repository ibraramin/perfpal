//! Perfpal binary entry point. All logic lives in the library crate so the
//! pipeline is integration-testable; `main` stays a thin shell around
//! [`perfpal::cli::run`] (exit-code mapping per modules.md §8.2).

fn main() {
    std::process::exit(perfpal::cli::run());
}
