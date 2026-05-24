//! `trackly` binary — Phase 1 scaffold.
//!
//! The full ordered lifecycle (WEBVIEW2 env-var → paths → config → tracing →
//! writer connection + PRAGMAs → user_version check → refinery → reader pool
//! → AppCtx → Tauri Builder) lands across Plans 02, 03, 04.
//! For Plan 01 the binary is invokable, accepts `--self-test`, and exits 0.

fn main() -> anyhow::Result<()> {
    let self_test = std::env::args().any(|a| a == "--self-test");
    if self_test {
        println!(
            "trackly self-test placeholder — Plan 02 wires paths, \
             Plan 03 wires migrations, Plan 04 wires AppCtx"
        );
        return Ok(());
    }
    println!("trackly Phase 1 scaffold — no UI yet");
    Ok(())
}
