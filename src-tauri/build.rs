fn main() {
    // Re-bake the Supabase secrets (read via option_env! in supabase.rs) when they
    // change, so cached builds don't keep stale compile-time values.
    println!("cargo:rerun-if-env-changed=SUPABASE_URL");
    println!("cargo:rerun-if-env-changed=SUPABASE_ANON_KEY");
    tauri_build::build()
}
