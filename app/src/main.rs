//! Lanzador de escritorio de Mi Garaje.
//!
//! Toda la interfaz vive en `lib.rs` para que la misma biblioteca se pueda
//! compilar como `.so` en Android (ver `android_main` allí).

fn main() -> eframe::Result<()> {
    mi_garaje::desktop_main()
}
