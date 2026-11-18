# MarketMaker: reglas del proyecto

- **Skeletons de carga**: cada pantalla tiene un skeleton con la forma de su diseño real (`ui::PanelSkeleton`, `ui::EditorSkeleton`, `ui::PageSkeleton` en `crates/ui/src/lib.rs`). Si cambiás el diseño de una pantalla, revisá y actualizá su skeleton en el mismo cambio. Nada de bloques grises genéricos (`h-96 w-full`).
- Antes de tocar interfaz, consultá la skill `modern-web-guidance` (con `DISABLE_TELEMETRY=1`) y reportá qué aportó.
