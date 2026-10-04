//! Bounded OpenType MATH glyph-assembly planning.
//!
//! The solver is direction-independent. It plans part repetition and connector
//! overlap in exact [`Dim`] units without allocating proportionally to the
//! requested target. Only the final validated solution is materialized.

use crate::font::{AssemblyPart, GlyphAssembly, MAX_ASSEMBLY_PARTS};
use crate::Dim;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssemblySolveError {
    Empty,
    NoExtender,
    InvalidPart,
    InvalidConnector,
    NonGrowing,
    NonMonotonic,
    OverBudget,
    Arithmetic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AssemblyPlacement {
    pub(crate) glyph_id: u16,
    /// Growth-direction distance from this part's origin to the next part's
    /// origin. For the final part this is its full advance.
    pub(crate) advance: Dim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AssemblySolution {
    pub(crate) placements: Vec<AssemblyPlacement>,
    pub(crate) advance: Dim,
    pub(crate) italic_correction: Dim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AssemblyRange {
    part_count: usize,
    connection_count: usize,
    min_advance: Dim,
    max_advance: Dim,
}

pub(crate) fn solve_glyph_assembly(
    assembly: &GlyphAssembly,
    target: &Dim,
) -> Result<AssemblySolution, AssemblySolveError> {
    validate_source(assembly)?;

    let fixed_count = assembly.parts.iter().filter(|part| !part.extender).count();
    let extender_count = assembly.parts.len() - fixed_count;
    if extender_count == 0 {
        return Err(AssemblySolveError::NoExtender);
    }

    let max_repeats = MAX_ASSEMBLY_PARTS
        .checked_sub(fixed_count)
        .ok_or(AssemblySolveError::OverBudget)?
        / extender_count;
    let first_repeat = if fixed_count == 0 { 1 } else { 0 };
    if max_repeats < first_repeat {
        return Err(AssemblySolveError::OverBudget);
    }

    // Extender runs are affine after the first included copy: increasing the
    // repetition count by one adds exactly one copy of every extender and one
    // same-part connection inside each extender run. Checking the 0->1
    // transition (when it exists) and the 1->2 delta therefore establishes the
    // monotonic interval used by the bounded binary search below.
    let first_range = range_for_repetitions(assembly, first_repeat)?;
    if first_range.part_count == 0 {
        return Err(AssemblySolveError::Empty);
    }

    if first_repeat == 0 && max_repeats >= 1 {
        let one = range_for_repetitions(assembly, 1)?;
        validate_growth_step(&first_range, &one)?;
    }
    if max_repeats >= 2 {
        let one = range_for_repetitions(assembly, 1)?;
        let two = range_for_repetitions(assembly, 2)?;
        validate_growth_step(&one, &two)?;

        if max_repeats >= 3 {
            let three = range_for_repetitions(assembly, 3)?;
            let min_delta_12 = two
                .min_advance
                .checked_sub(&one.min_advance)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            let min_delta_23 = three
                .min_advance
                .checked_sub(&two.min_advance)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            let max_delta_12 = two
                .max_advance
                .checked_sub(&one.max_advance)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            let max_delta_23 = three
                .max_advance
                .checked_sub(&two.max_advance)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            if min_delta_12 != min_delta_23 || max_delta_12 != max_delta_23 {
                return Err(AssemblySolveError::NonMonotonic);
            }
        }
    }

    let last_range = range_for_repetitions(assembly, max_repeats)?;
    if target > &last_range.max_advance {
        return Err(AssemblySolveError::OverBudget);
    }

    let repeat = if target <= &first_range.max_advance {
        first_repeat
    } else {
        let mut low = first_repeat + 1;
        let mut high = max_repeats;
        while low < high {
            let mid = low + (high - low) / 2;
            let range = range_for_repetitions(assembly, mid)?;
            if target <= &range.max_advance {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        low
    };

    let range = range_for_repetitions(assembly, repeat)?;
    let desired = target.max_ref(&range.min_advance);
    let growable = range
        .max_advance
        .checked_sub(&range.min_advance)
        .map_err(|_| AssemblySolveError::Arithmetic)?;
    let extra = desired
        .checked_sub(&range.min_advance)
        .map_err(|_| AssemblySolveError::Arithmetic)?;
    let ratio = if growable.is_zero() {
        Dim::zero()
    } else {
        extra
            .checked_div(&growable)
            .map_err(|_| AssemblySolveError::Arithmetic)?
    };

    let placements = materialize(assembly, repeat, &ratio, range.part_count)?;
    let actual = placements.iter().try_fold(Dim::zero(), |total, placement| {
        total.checked_add(&placement.advance)
    });
    let actual = actual.map_err(|_| AssemblySolveError::Arithmetic)?;
    if actual != desired {
        return Err(AssemblySolveError::NonMonotonic);
    }

    Ok(AssemblySolution {
        placements,
        advance: actual,
        italic_correction: assembly.italic_correction.clone(),
    })
}

fn validate_source(assembly: &GlyphAssembly) -> Result<(), AssemblySolveError> {
    if assembly.parts.is_empty() {
        return Err(AssemblySolveError::Empty);
    }
    if assembly.parts.len() > MAX_ASSEMBLY_PARTS {
        return Err(AssemblySolveError::OverBudget);
    }

    for part in &assembly.parts {
        if part.full_advance <= Dim::zero()
            || part.start_connector < Dim::zero()
            || part.end_connector < Dim::zero()
            || part.start_connector > part.full_advance
            || part.end_connector > part.full_advance
        {
            return Err(AssemblySolveError::InvalidPart);
        }
    }
    if assembly.min_connector_overlap < Dim::zero() {
        return Err(AssemblySolveError::InvalidConnector);
    }
    Ok(())
}

fn validate_growth_step(
    previous: &AssemblyRange,
    next: &AssemblyRange,
) -> Result<(), AssemblySolveError> {
    if next.part_count <= previous.part_count || next.connection_count < previous.connection_count {
        return Err(AssemblySolveError::NonMonotonic);
    }
    if next.min_advance < previous.min_advance || next.max_advance < previous.max_advance {
        return Err(AssemblySolveError::NonMonotonic);
    }
    if next.max_advance == previous.max_advance {
        return Err(AssemblySolveError::NonGrowing);
    }
    Ok(())
}

fn range_for_repetitions(
    assembly: &GlyphAssembly,
    repeat: usize,
) -> Result<AssemblyRange, AssemblySolveError> {
    let mut part_count = 0usize;
    let mut connection_count = 0usize;
    let mut full = Dim::zero();
    let mut max_overlap_total = Dim::zero();
    let mut previous_end: Option<&Dim> = None;

    for part in &assembly.parts {
        let count = if part.extender { repeat } else { 1 };
        if count == 0 {
            continue;
        }

        part_count = part_count
            .checked_add(count)
            .ok_or(AssemblySolveError::OverBudget)?;
        if part_count > MAX_ASSEMBLY_PARTS {
            return Err(AssemblySolveError::OverBudget);
        }

        let count_i64 = i64::try_from(count).map_err(|_| AssemblySolveError::OverBudget)?;
        let block_full = part
            .full_advance
            .checked_mul(&Dim::from_i64(count_i64))
            .map_err(|_| AssemblySolveError::Arithmetic)?;
        full = full
            .checked_add(&block_full)
            .map_err(|_| AssemblySolveError::Arithmetic)?;

        if let Some(end) = previous_end {
            let overlap = legal_max_overlap(end, &part.start_connector, assembly)?;
            max_overlap_total = max_overlap_total
                .checked_add(&overlap)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            connection_count = connection_count
                .checked_add(1)
                .ok_or(AssemblySolveError::OverBudget)?;
        }

        if count > 1 {
            let overlap = legal_max_overlap(&part.end_connector, &part.start_connector, assembly)?;
            let repeated_connections = count - 1;
            let repeated_i64 =
                i64::try_from(repeated_connections).map_err(|_| AssemblySolveError::OverBudget)?;
            let repeated_overlap = overlap
                .checked_mul(&Dim::from_i64(repeated_i64))
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            max_overlap_total = max_overlap_total
                .checked_add(&repeated_overlap)
                .map_err(|_| AssemblySolveError::Arithmetic)?;
            connection_count = connection_count
                .checked_add(repeated_connections)
                .ok_or(AssemblySolveError::OverBudget)?;
        }

        previous_end = Some(&part.end_connector);
    }

    if part_count == 0 {
        return Ok(AssemblyRange {
            part_count,
            connection_count,
            min_advance: Dim::zero(),
            max_advance: Dim::zero(),
        });
    }

    let connection_i64 =
        i64::try_from(connection_count).map_err(|_| AssemblySolveError::OverBudget)?;
    let min_overlap_total = assembly
        .min_connector_overlap
        .checked_mul(&Dim::from_i64(connection_i64))
        .map_err(|_| AssemblySolveError::Arithmetic)?;
    let min_advance = full
        .checked_sub(&max_overlap_total)
        .map_err(|_| AssemblySolveError::Arithmetic)?;
    let max_advance = full
        .checked_sub(&min_overlap_total)
        .map_err(|_| AssemblySolveError::Arithmetic)?;
    if min_advance < Dim::zero() || max_advance < min_advance {
        return Err(AssemblySolveError::InvalidConnector);
    }

    Ok(AssemblyRange {
        part_count,
        connection_count,
        min_advance,
        max_advance,
    })
}

fn legal_max_overlap(
    previous_end: &Dim,
    next_start: &Dim,
    assembly: &GlyphAssembly,
) -> Result<Dim, AssemblySolveError> {
    let maximum = previous_end.min_ref(next_start);
    if maximum < assembly.min_connector_overlap {
        return Err(AssemblySolveError::InvalidConnector);
    }
    Ok(maximum)
}

fn materialize(
    assembly: &GlyphAssembly,
    repeat: usize,
    ratio: &Dim,
    part_count: usize,
) -> Result<Vec<AssemblyPlacement>, AssemblySolveError> {
    if part_count > MAX_ASSEMBLY_PARTS {
        return Err(AssemblySolveError::OverBudget);
    }

    let mut placements = Vec::with_capacity(part_count);
    let mut previous: Option<&AssemblyPart> = None;

    for part in &assembly.parts {
        let count = if part.extender { repeat } else { 1 };
        for _ in 0..count {
            if let Some(prev) = previous {
                let maximum =
                    legal_max_overlap(&prev.end_connector, &part.start_connector, assembly)?;
                let slack = maximum
                    .checked_sub(&assembly.min_connector_overlap)
                    .map_err(|_| AssemblySolveError::Arithmetic)?;
                let opened = slack
                    .checked_mul(ratio)
                    .map_err(|_| AssemblySolveError::Arithmetic)?;
                let overlap = maximum
                    .checked_sub(&opened)
                    .map_err(|_| AssemblySolveError::Arithmetic)?;
                let advance = prev
                    .full_advance
                    .checked_sub(&overlap)
                    .map_err(|_| AssemblySolveError::Arithmetic)?;
                placements.push(AssemblyPlacement {
                    glyph_id: prev.glyph_id,
                    advance,
                });
            }
            previous = Some(part);
        }
    }

    let last = previous.ok_or(AssemblySolveError::Empty)?;
    placements.push(AssemblyPlacement {
        glyph_id: last.glyph_id,
        advance: last.full_advance.clone(),
    });
    if placements.len() != part_count {
        return Err(AssemblySolveError::NonMonotonic);
    }
    Ok(placements)
}

#[cfg(test)]
mod tests {
    use super::{
        range_for_repetitions, solve_glyph_assembly, AssemblySolveError, MAX_ASSEMBLY_PARTS,
    };
    use crate::font::{AssemblyPart, GlyphAssembly};
    use crate::Dim;

    fn d(value: i64) -> Dim {
        Dim::from_i64(value)
    }

    fn part(glyph_id: u16, start: i64, end: i64, advance: i64, extender: bool) -> AssemblyPart {
        AssemblyPart {
            glyph_id,
            start_connector: d(start),
            end_connector: d(end),
            full_advance: d(advance),
            extender,
        }
    }

    fn assembly(parts: Vec<AssemblyPart>, min_overlap: i64) -> GlyphAssembly {
        GlyphAssembly {
            italic_correction: Dim::ratio(1, 10).unwrap(),
            min_connector_overlap: d(min_overlap),
            parts,
        }
    }

    #[test]
    fn no_extender_is_rejected_as_non_growing_construction() {
        let input = assembly(vec![part(1, 0, 2, 5, false), part(2, 2, 0, 5, false)], 1);
        assert_eq!(
            solve_glyph_assembly(&input, &d(8)),
            Err(AssemblySolveError::NoExtender)
        );
    }

    #[test]
    fn one_extender_can_be_skipped_then_uses_overlap_range_exactly() {
        let input = assembly(
            vec![
                part(1, 0, 3, 5, false),
                part(2, 3, 3, 4, true),
                part(3, 3, 0, 5, false),
            ],
            1,
        );
        let zero = range_for_repetitions(&input, 0).unwrap();
        assert_eq!(zero.min_advance, d(7));
        assert_eq!(zero.max_advance, d(9));

        let one = range_for_repetitions(&input, 1).unwrap();
        assert_eq!(one.min_advance, d(8));
        assert_eq!(one.max_advance, d(12));

        let compact = solve_glyph_assembly(&input, &d(7)).unwrap();
        assert_eq!(compact.advance, d(7));
        let compact_ids: Vec<u16> = compact
            .placements
            .iter()
            .map(|placement| placement.glyph_id)
            .collect();
        assert_eq!(compact_ids, vec![1, 3]);
        assert_eq!(compact.placements[0].advance, d(2));
        assert_eq!(compact.placements[1].advance, d(5));

        let expanded = solve_glyph_assembly(&input, &d(12)).unwrap();
        assert_eq!(expanded.advance, d(12));
        assert_eq!(expanded.placements[0].advance, d(4));
        assert_eq!(expanded.placements[1].advance, d(3));

        let interior = solve_glyph_assembly(&input, &d(10)).unwrap();
        assert_eq!(interior.advance, d(10));
        assert_eq!(interior.placements[0].advance, d(3));
        assert_eq!(interior.placements[1].advance, d(2));
        assert_eq!(interior.italic_correction, Dim::ratio(1, 10).unwrap());
    }

    #[test]
    fn multiple_extenders_and_positions_repeat_in_uniform_rounds() {
        let input = assembly(
            vec![
                part(1, 0, 2, 4, false),
                part(2, 2, 2, 3, true),
                part(3, 2, 2, 4, false),
                part(4, 2, 2, 3, true),
                part(5, 2, 0, 4, false),
            ],
            1,
        );
        let solution = solve_glyph_assembly(&input, &d(17)).unwrap();
        let ids: Vec<u16> = solution
            .placements
            .iter()
            .map(|placement| placement.glyph_id)
            .collect();
        assert_eq!(ids, vec![1, 2, 2, 3, 4, 4, 5]);
        assert!(solution.advance >= d(17));
    }

    #[test]
    fn multiple_repetitions_choose_the_first_legal_round_that_reaches_target() {
        let input = assembly(
            vec![
                part(1, 0, 2, 4, false),
                part(2, 2, 2, 3, true),
                part(3, 2, 0, 4, false),
            ],
            1,
        );
        let one = range_for_repetitions(&input, 1).unwrap();
        let two = range_for_repetitions(&input, 2).unwrap();
        assert!(one.max_advance < two.max_advance);
        let target = one
            .max_advance
            .checked_add(&Dim::ratio(1, 2).unwrap())
            .unwrap();
        let solution = solve_glyph_assembly(&input, &target).unwrap();
        assert_eq!(solution.placements.len(), 4);
        assert_eq!(solution.advance, target);
    }

    #[test]
    fn zero_net_growth_and_invalid_connectors_are_rejected() {
        let zero_growth = assembly(vec![part(1, 0, 4, 4, false), part(2, 4, 4, 4, true)], 4);
        assert_eq!(
            solve_glyph_assembly(&zero_growth, &d(20)),
            Err(AssemblySolveError::NonGrowing)
        );

        let invalid_connector = assembly(
            vec![
                part(1, 0, 1, 4, false),
                part(2, 1, 1, 4, true),
                part(3, 1, 0, 4, false),
            ],
            2,
        );
        assert_eq!(
            solve_glyph_assembly(&invalid_connector, &d(20)),
            Err(AssemblySolveError::InvalidConnector)
        );
    }

    #[test]
    fn invalid_connector_lengths_and_exact_arithmetic_boundary_fail_deterministically() {
        let invalid_part = assembly(vec![part(1, 0, 5, 4, false), part(2, 4, 4, 4, true)], 1);
        assert_eq!(
            solve_glyph_assembly(&invalid_part, &d(10)),
            Err(AssemblySolveError::InvalidPart)
        );

        let huge = Dim::parse("170141183460469231731687303715884105727").unwrap();
        let overflow = GlyphAssembly {
            italic_correction: Dim::zero(),
            min_connector_overlap: Dim::zero(),
            parts: vec![
                AssemblyPart {
                    glyph_id: 1,
                    start_connector: Dim::zero(),
                    end_connector: Dim::zero(),
                    full_advance: huge,
                    extender: false,
                },
                part(2, 0, 0, 1, true),
            ],
        };
        assert_eq!(
            solve_glyph_assembly(&overflow, &d(1)),
            Err(AssemblySolveError::Arithmetic)
        );
    }

    #[test]
    fn non_monotonic_range_transition_is_classified_as_unusable() {
        let previous = super::AssemblyRange {
            part_count: 2,
            connection_count: 1,
            min_advance: d(10),
            max_advance: d(12),
        };
        let next = super::AssemblyRange {
            part_count: 3,
            connection_count: 2,
            min_advance: d(9),
            max_advance: d(13),
        };
        assert_eq!(
            super::validate_growth_step(&previous, &next),
            Err(AssemblySolveError::NonMonotonic)
        );
    }

    #[test]
    fn part_budget_accepts_exact_boundary_and_rejects_next_required_round() {
        let input = assembly(vec![part(1, 0, 1, 2, false), part(2, 1, 1, 2, true)], 1);
        let max_repeat = MAX_ASSEMBLY_PARTS - 1;
        let boundary = range_for_repetitions(&input, max_repeat).unwrap();
        assert_eq!(boundary.part_count, MAX_ASSEMBLY_PARTS);

        let exact = solve_glyph_assembly(&input, &boundary.max_advance).unwrap();
        assert_eq!(exact.placements.len(), MAX_ASSEMBLY_PARTS);

        let beyond = boundary
            .max_advance
            .checked_add(&Dim::ratio(1, 2).unwrap())
            .unwrap();
        assert_eq!(
            solve_glyph_assembly(&input, &beyond),
            Err(AssemblySolveError::OverBudget)
        );
    }

    #[test]
    fn solution_is_axis_agnostic_for_horizontal_and_vertical_consumers() {
        let input = assembly(
            vec![
                part(10, 0, 2, 4, false),
                part(11, 2, 2, 3, true),
                part(12, 2, 0, 4, false),
            ],
            1,
        );
        let target = Dim::ratio(19, 2).unwrap();
        let horizontal = solve_glyph_assembly(&input, &target).unwrap();
        let vertical = solve_glyph_assembly(&input, &target).unwrap();
        assert_eq!(horizontal, vertical);
        assert_eq!(horizontal.advance, target);
    }
}
