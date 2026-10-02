## Tests / Windows

- [ ] En Windows, ejecutar también los tests de fuente/layout compatibles con Cambria Math usando la fuente instalada por el sistema (`%WINDIR%\\Fonts\\cambria.ttc`). Resolver explícitamente la cara `Cambria Math` dentro de la colección TTC en lugar de asumir un índice fijo, y no incorporar ni redistribuir la fuente como fixture del repositorio.

## Rendimiento / GSUB E8 — optimización adicional diferida hasta E11

- [ ] Retomar únicamente la optimización residual de GSUB en E11 (`docs/FONT_ACCESS.md`) o al finalizar la implementación funcional del roadmap. El plan GSUB perezoso por `MathFontView` medido el 2026-10-02 queda adoptado en E8 porque produjo una mejora material y preservó los contratos funcionales; no seguir iterándolo ahora. Queda pendiente explicar y, si la evidencia multi-font lo justifica, reducir el residual `plain-*`. Baseline: `24d6bde` (E7). Evidencia emparejada inicial y posterior al plan: 9 pares alternando E7→E8 / E8→E7 por corrida, Windows 11 x86_64, i5-13600KF, Rust/Cargo 1.95.0. Los targets absolutos pasan; no deben usarse para ocultar la regresión relativa.

  | Métrica | E7 mediana | E8 mediana | ratio E8/E7 mediano | p95 del ratio |
  | --- | ---: | ---: | ---: | ---: |
  | construct STIX Two Math | 563 ns | 566 ns | 1.0071x | 1.4420x |
  | parse `\frac{1}{2}` | 3.961 µs | 3.935 µs | 0.9972x | 1.0163x |
  | parse full display equation | 6.369 µs | 6.420 µs | 1.0062x | 1.0954x |
  | layout `\frac{1}{2}` | 6.518 µs | 9.505 µs | 1.4699x | 1.4956x |
  | layout full display equation | 28.047 µs | 39.034 µs | 1.4066x | 1.4323x |
  | plain-1 | 2.939 µs | 3.237 µs | 1.0924x | 1.1236x |
  | plain-32 | 36.953 µs | 48.454 µs | 1.3058x | 1.3318x |
  | ssty-level1 | 4.732 µs | 6.528 µs | 1.3799x | 1.4016x |
  | ssty-level2 | 7.366 µs | 10.563 µs | 1.4346x | 1.4490x |
  | ssty-dense | 34.068 µs | 49.643 µs | 1.4643x | 1.5073x |
  | ssty-deep-dense | 33.328 µs | 50.119 µs | 1.5066x | 1.5737x |

  Revisar causalmente, en este orden:

  1. Comparar el camino de layout E8 contra `24d6bde` y separar el costo GSUB de cualquier otro cambio E8. Los controles de parseo son ~1.00x, por lo que el costo está después del parser.
  2. Explicar la regresión de `plain-1` y `plain-32`. `math_gsub_glyph_id` ya retorna inmediatamente cuando `script_level` no es 1/2 y `MathGsubContext::None`; por tanto, esos casos impiden atribuir toda la regresión a una simple ausencia de fast path GSUB. Medir/perfilar qué trabajo adicional queda en el camino de layout de texto normal y por qué escala con 32 glyphs.
  3. Tomar como baseline actual el plan GSUB perezoso por operación ya adoptado en `MathFontView`: resuelve una vez los lookups alcanzables de `ssty`, `flac` y `dtls`, conserva el orden global de `LookupList` y los reutiliza durante el layout. No seguir optimizando esa ruta en E8.
  4. En E11, medir separadamente el costo residual del plan adoptado: preparación por operación, costo por glyph, asignaciones y memoria temporal, con especial atención a `plain-*`. Sólo reabrir la representación o promover un índice persistente de `MathFont` si la medición multi-font demuestra un beneficio neto.
  5. Preservar exactamente la semántica E8 al optimizar: sólo `ScriptList["math"]`, sólo `DefaultLangSys`, feature requerida + feature indices alcanzables, `ssty` niveles 1/2, Alternate Substitution y fallback Single permitido, `flac`/`dtls` sólo Single, y orden compartido de lookups cuando varias features son aplicables al mismo glyph. Los tests de `tests/math_gsub_features.rs` son contratos y no deben relajarse para ganar rendimiento.
  6. Medir por separado costo de preparación por operación, costo por glyph, asignaciones y memoria temporal. Evitar una optimización que reduzca tiempo por glyph a costa de trabajo fijo mayor en fórmulas pequeñas sin evidencia neta favorable.
  7. Repetir exactamente la medición emparejada E7/E8 después del cambio, con el mismo corpus, 9 pares alternados y censo completo. Los controles de construcción/parseo deben seguir comportándose como controles; reportar mediana y p95 de ratios y los targets absolutos por separado. No declarar resuelta la regresión si la mejora sólo aparece en una corrida no emparejada o queda mezclada con ruido de controles.
  8. Después de la última mutación semántica/performance, ejecutar los tests GSUB focales, el gate Rust completo, Clippy, oracle canónico y stress correspondiente a font/layout. La evidencia debe corresponder al estado exacto que se cierre.
  9. Antes de eliminar probes/runners temporales, registrar de forma durable la hipótesis, mecanismo medido, resultados, decisión adoptada o rechazada, efecto de memoria y condición para reabrirla; `docs/FONT_ACCESS.md` es el destino previsto por E11 para esa decisión de indexación/precomputación.

  Experimento del 2026-10-02: el plan GSUB perezoso por `MathFontView` redujo materialmente la regresión de `layout \frac{1}{2}` de ~1.47x a 1.0326x, `layout full display equation` de ~1.41x a 1.1061x y los casos `ssty-*` a ~1.07–1.12x. No resolvió el costo `plain-*`: `plain-1` quedó en 1.1150x y `plain-32` en 1.2888x. Decisión adoptada: conservar el plan por operación en E8 por la mejora medida y por mantener verdes los contratos GSUB; no realizar más optimización en este milestone. E11 debe explicar primero el residual `plain-*` y decidir, con medición multi-font y memoria/costo de preparación, si corresponde otra representación o un índice persistente.
