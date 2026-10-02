## Tests / Windows

- [ ] En Windows, ejecutar también los tests de fuente/layout compatibles con Cambria Math usando la fuente instalada por el sistema (`%WINDIR%\\Fonts\\cambria.ttc`). Resolver explícitamente la cara `Cambria Math` dentro de la colección TTC en lugar de asumir un índice fijo, y no incorporar ni redistribuir la fuente como fixture del repositorio.

## Rendimiento / GSUB E8 — resuelto en E11

- [x] La regresión relativa de layout registrada en E8 fue investigada y ya no se reproduce en `c351daf`: nueve pares alternados contra E7 (`24d6bde`) dieron ratios CURRENT/E7 entre `0.9071x` y `0.9928x` en los workloads `plain-*`, `ssty-*`, fracción y ecuación display. La auditoría de fuente identificó una doble extracción de métricas por glyph en el camino E8 anterior a E10; E10 la eliminó al resolver sólo `glyph_index` antes de GSUB y extraer métricas una vez después. El experimento de allocations no justificó rediseñar `MathBox`/rows, y las variantes de fast-path/inlining tampoco dieron una mejora runtime consistente. E11 mantiene únicamente el `MathGsubPlan` perezoso por operación y no añade índices persistentes ni nueva precomputación. Evidencia, metodología, hashes y decisión: `docs/FONT_ACCESS.md`.
