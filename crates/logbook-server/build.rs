//! Rebuilds the crate when a migration changes. `sqlx::migrate!` embeds the files
//! at compile time, and Cargo does not see a new file in `migrations/` by itself.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
