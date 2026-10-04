#!/usr/bin/env python3
"""LuaLaTeX differential oracle for TeXpose mathematical layout."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, replace
from fractions import Fraction
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parent.parent
PROFILE_REVISION = "oracle-v9"
CANONICAL_CENSUS_SHA256 = "d1d1e356e5a4f426ebf603ed6be5cb134dca3bebe9be7f6b49938ed5ada4ddf1"
STRESS_CENSUS_SHA256 = "383828f9f734c65e67ce7b9a4f12f501fc2951825ed8ffd1ff6fca945b5e29b4"
CANONICAL_ALIAS_CENSUS_SHA256 = CANONICAL_CENSUS_SHA256
STRESS_ALIAS_CENSUS_SHA256 = "1eae50562863db2a26d7c28c96c78a82c3a47649348cb277fc7f17b7578f3b71"
KNOWN_CAPABILITIES = frozenset({"math-font", "canonical-corpus", "stress-corpus"})
REFERENCE_ENVIRONMENT_SHA256 = "b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf"
STIX_CANONICAL_TRACE_GLYPH_MISMATCHES = (
    ("display-sum-limits", ((0, 1647, 4437), (1, 4437, 1647))),
    ("display-sum", ((0, 1647, 4437), (1, 4437, 1647))),
    ("accent-hat-j", ((0, 3309, 732), (1, 4800, 3309))),
    ("accent-widehat-j", ((0, 3309, 732), (1, 1395, 3309))),
    (
        "accent-widehat-xyz",
        ((0, 3323, 1398), (1, 3324, 3323), (2, 3325, 3324), (3, 1399, 3325)),
    ),
    (
        "accent-widetilde-xyz",
        ((0, 3323, 1408), (1, 3324, 3323), (2, 3325, 3324), (3, 1409, 3325)),
    ),
    (
        "accent-widehat-script",
        ((0, 3354, 732), (1, 4275, 3354), (2, 4430, 4275), (3, 1395, 4430)),
    ),
)

LIBERTINUS_CANONICAL_TRACE_GLYPH_MISMATCHES = (
    ("display-sum-limits", ((0, 3985, 2758), (1, 2758, 3985))),
    ("display-sum", ((0, 3985, 2758), (1, 2758, 3985))),
    ("accent-hat-j", ((0, 2729, 701), (1, 701, 2729))),
    ("accent-widehat-j", ((0, 2729, 701), (1, 4071, 2729))),
    (
        "accent-widehat-xyz",
        ((0, 2743, 4074), (1, 2744, 2743), (2, 2745, 2744), (3, 4075, 2745)),
    ),
    (
        "accent-widetilde-xyz",
        ((0, 2743, 4216), (1, 2744, 2743), (2, 2745, 2744), (3, 4217, 2745)),
    ),
    (
        "accent-widehat-script",
        ((0, 2768, 701), (1, 19, 2768), (2, 2753, 19), (3, 4071, 2753)),
    ),
)

FIRA_CANONICAL_TRACE_GLYPH_MISMATCHES = (
    ("display-sum-limits", ((0, 1584, 1216), (1, 1216, 1584))),
    ("display-sum", ((0, 1584, 1216), (1, 1216, 1584))),
    ("accent-hat-j", ((0, 1187, 285), (1, 285, 1187))),
    ("accent-widehat-j", ((0, 1187, 285), (1, 285, 1187))),
    (
        "accent-widehat-xyz",
        ((0, 1201, 285), (1, 1202, 1201), (2, 1203, 1202), (3, 285, 1203)),
    ),
    (
        "accent-widetilde-xyz",
        ((0, 1201, 286), (1, 1202, 1201), (2, 1203, 1202), (3, 286, 1203)),
    ),
    (
        "accent-widehat-script",
        ((0, 1226, 285), (1, 19, 1226), (2, 1211, 19), (3, 285, 1211)),
    ),
)


class OracleError(RuntimeError):
    pass


@dataclass(frozen=True)
class Deviation:
    geometry_ceiling: float | None
    allow_structure: bool
    note: str


@dataclass(frozen=True)
class MathProfile:
    name: str
    fixture: str
    sha256: str
    face_index: int
    required_capabilities: tuple[str, ...]
    capability_exclusions: tuple[tuple[str, str], ...]
    canonical_measurements: int
    canonical_aliases: int
    canonical_census_sha256: str
    canonical_alias_census_sha256: str
    canonical_tolerance: float
    stress_measurements: int
    stress_aliases: int
    stress_census_sha256: str
    stress_alias_census_sha256: str
    stress_tolerance: float
    documented_deviations: tuple[tuple[str, Deviation], ...]
    canonical_trace_glyph_mismatches: tuple[tuple[str, tuple[tuple[int, int, int], ...]], ...]
    canonical_trace_deviations: tuple[tuple[str, float], ...]
    reference_environment_sha256: str | None


@dataclass(frozen=True)
class RunSpec:
    name: str
    font_path: Path
    font_sha256: str
    face_index: int
    revision: str
    measurement_count: int
    alias_count: int
    census_sha256: str
    alias_census_sha256: str
    tolerance: float
    documented_deviations: tuple[tuple[str, Deviation], ...]
    canonical_trace_glyph_mismatches: tuple[tuple[str, tuple[tuple[int, int, int], ...]], ...]
    canonical_trace_deviations: tuple[tuple[str, float], ...]
    reference_environment_sha256: str | None
    contractual_profile: bool
    collection: bool


@dataclass(frozen=True)
class ProbeMeta:
    profile: str
    revision: str
    font_sha256: str
    face_index: int


@dataclass(frozen=True)
class ReferenceFingerprint:
    engine: str
    distribution: str
    latex: str
    unicode_math: str
    fontspec: str
    amsmath: str
    font_sha256: str
    face_index: int
    profile: str
    revision: str
    census_sha256: str
    alias_census_sha256: str

    def environment_sha256(self) -> str:
        payload = json.dumps(
            {
                "engine": self.engine,
                "distribution": self.distribution,
                "latex": self.latex,
                "unicode_math": self.unicode_math,
                "fontspec": self.fontspec,
                "amsmath": self.amsmath,
            },
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("ascii")
        return hashlib.sha256(payload).hexdigest()


@dataclass(frozen=True)
class CaseDelta:
    name: str
    family: str
    size: int
    width: float
    ascent: float
    descent: float
    texpose_glyphs: int
    reference_glyphs: int
    texpose_rules: int
    reference_rules: int

    @property
    def maximum(self) -> float:
        return max(self.width, self.ascent, self.descent)

    @property
    def maximum_dimension(self) -> str:
        values = (
            (self.width, "width"),
            (self.ascent, "ascent"),
            (self.descent, "descent"),
        )
        return max(values, key=lambda item: item[0])[1]

    @property
    def structural_mismatch(self) -> bool:
        return (
            self.texpose_glyphs != self.reference_glyphs
            or self.texpose_rules != self.reference_rules
        )


@dataclass(frozen=True)
class GlyphMismatch:
    index: int
    texpose_glyph_id: int
    reference_glyph_id: int


@dataclass(frozen=True)
class TraceCaseDelta:
    name: str
    family: str
    size: int
    primitive_count: int
    topology_mismatch: str | None
    glyph_mismatches: tuple[GlyphMismatch, ...]
    glyph_multiset_matches: bool | None
    geometry_alignment: str | None
    maximum: float
    maximum_field: str | None
    maximum_index: int | None
    maximum_reference_index: int | None

    @property
    def topology_matches(self) -> bool:
        return self.topology_mismatch is None

    @property
    def glyphs_match(self) -> bool:
        return self.topology_matches and not self.glyph_mismatches

    @property
    def geometry_comparable(self) -> bool:
        return self.topology_matches and self.geometry_alignment is not None


PROFILES: dict[str, MathProfile] = {
    "stix": MathProfile(
        name="stix",
        fixture="tests/fixtures/fonts/stix-two-math/STIXTwoMath-Regular.otf",
        sha256="f2076b9f1676438439dd41e23676f5ab99056e83d6b8f8c27841591ef2ccfa72",
        face_index=0,
        required_capabilities=("math-font", "canonical-corpus", "stress-corpus"),
        capability_exclusions=(),
        canonical_measurements=21,
        canonical_aliases=21,
        canonical_census_sha256=CANONICAL_CENSUS_SHA256,
        canonical_alias_census_sha256=CANONICAL_ALIAS_CENSUS_SHA256,
        canonical_tolerance=0.05,
        stress_measurements=89,
        stress_aliases=94,
        stress_census_sha256=STRESS_CENSUS_SHA256,
        stress_alias_census_sha256=STRESS_ALIAS_CENSUS_SHA256,
        stress_tolerance=0.05,
        documented_deviations=(
            (
                "accent-widehat-j",
                Deviation(
                    geometry_ceiling=0.0801,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.079999em ascent delta because "
                        "LuaLaTeX retains the base circumflex while G5 follows the MATH "
                        "advanceMeasurement construction. The ceiling freezes that external "
                        "difference without weakening horizontal-variant selection."
                    ),
                ),
            ),
        ),
        canonical_trace_glyph_mismatches=STIX_CANONICAL_TRACE_GLYPH_MISMATCHES,
        canonical_trace_deviations=(),
        reference_environment_sha256=REFERENCE_ENVIRONMENT_SHA256,
    ),
    "libertinus": MathProfile(
        name="libertinus",
        fixture="tests/fixtures/fonts/libertinus-math/LibertinusMath-Regular.otf",
        sha256="e81bd44acbb7119c8f00128b36fecc5d980e10d2450a226ba52402ccf4da9d32",
        face_index=0,
        required_capabilities=("math-font", "canonical-corpus", "stress-corpus"),
        capability_exclusions=(),
        canonical_measurements=21,
        canonical_aliases=21,
        canonical_census_sha256=CANONICAL_CENSUS_SHA256,
        canonical_alias_census_sha256=CANONICAL_ALIAS_CENSUS_SHA256,
        canonical_tolerance=0.05,
        stress_measurements=89,
        stress_aliases=94,
        stress_census_sha256=STRESS_CENSUS_SHA256,
        stress_alias_census_sha256=STRESS_ALIAS_CENSUS_SHA256,
        stress_tolerance=0.05,
        documented_deviations=(
            (
                "display-nested-fraction",
                Deviation(
                    geometry_ceiling=0.0625,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.062386em width delta; "
                        "Phase G owns fraction geometry. The ceiling freezes the "
                        "observed divergence rather than accepting it as correct."
                    ),
                ),
            ),
            (
                "accent-widehat-j",
                Deviation(
                    geometry_ceiling=0.0571,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.056999em ascent delta because "
                        "LuaLaTeX retains the base circumflex while G5 follows the MATH "
                        "advanceMeasurement construction. The ceiling freezes that external "
                        "difference without weakening horizontal-variant selection."
                    ),
                ),
            ),
        ),
        canonical_trace_glyph_mismatches=LIBERTINUS_CANONICAL_TRACE_GLYPH_MISMATCHES,
        canonical_trace_deviations=(
            ("display-nested-fraction", 0.0625),
            ("text-scripts", 0.0811),
        ),
        reference_environment_sha256=REFERENCE_ENVIRONMENT_SHA256,
    ),
    "fira": MathProfile(
        name="fira",
        fixture="tests/fixtures/fonts/fira-math/FiraMath-Regular.otf",
        sha256="2028cbd3dd4d8c0cf1608520eb4759956a83a67931d7b6d8e7c313520186e35b",
        face_index=0,
        required_capabilities=("math-font", "canonical-corpus", "stress-corpus"),
        capability_exclusions=(),
        canonical_measurements=21,
        canonical_aliases=21,
        canonical_census_sha256=CANONICAL_CENSUS_SHA256,
        canonical_alias_census_sha256=CANONICAL_ALIAS_CENSUS_SHA256,
        canonical_tolerance=0.05,
        stress_measurements=89,
        stress_aliases=94,
        stress_census_sha256=STRESS_CENSUS_SHA256,
        stress_alias_census_sha256=STRESS_ALIAS_CENSUS_SHA256,
        stress_tolerance=0.05,
        documented_deviations=(),
        canonical_trace_glyph_mismatches=FIRA_CANONICAL_TRACE_GLYPH_MISMATCHES,
        canonical_trace_deviations=(),
        reference_environment_sha256=REFERENCE_ENVIRONMENT_SHA256,
    ),
}


def fail(message: str) -> None:
    raise OracleError(message)


def require_file(path: Path) -> None:
    if not path.is_file():
        fail(f"required file is missing: {path}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_float(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError:
        fail(f"expected finite nonnegative measurement: {value}")
    if not math.isfinite(parsed) or parsed < 0:
        fail(f"expected finite nonnegative measurement: {value}")
    return parsed


def parse_int(value: str, label: str) -> int:
    try:
        parsed = int(value)
    except ValueError:
        fail(f"expected integer {label}: {value}")
    if parsed < 0:
        fail(f"expected nonnegative integer {label}: {value}")
    return parsed



I128_MIN = -(1 << 127)
I128_MAX = (1 << 127) - 1
MAX_TEX_DIM = (1 << 31) - 1


def parse_dim_ratio(value: str, label: str) -> Fraction:
    match = re.fullmatch(r"(-?[0-9]+)/([1-9][0-9]*)", value)
    if match is None:
        fail(f"invalid exact Dim {label}: {value}")
    num = int(match.group(1))
    den = int(match.group(2))
    if not I128_MIN <= num <= I128_MAX or not 1 <= den <= I128_MAX:
        fail(f"exact Dim {label} is outside i128 range: {value}")
    if math.gcd(abs(num), den) != 1:
        fail(f"noncanonical exact Dim {label}: {value}")
    return Fraction(num, den)


def parse_signed_int(value: str, label: str) -> int:
    try:
        parsed = int(value)
    except ValueError:
        fail(f"expected integer {label}: {value}")
    if not -MAX_TEX_DIM <= parsed <= MAX_TEX_DIM:
        fail(f"integer {label} is outside TeX dimension range: {value}")
    return parsed

def utf8hex(value: str) -> str:
    try:
        return bytes.fromhex(value).decode("utf-8")
    except (ValueError, UnicodeDecodeError) as error:
        fail(f"invalid UTF-8 hex: {error}")


def census_sha256(names: Iterable[str]) -> str:
    return hashlib.sha256(",".join(names).encode("ascii")).hexdigest()


def validate_profile(profile: MathProfile) -> None:
    if not re.fullmatch(r"[a-z0-9_-]+", profile.name):
        fail(f"invalid profile name: {profile.name}")
    if profile.face_index < 0:
        fail(f"negative face index in profile {profile.name}")
    if not re.fullmatch(r"[0-9a-f]{64}", profile.sha256):
        fail(f"invalid fixture hash in profile {profile.name}")
    if not math.isfinite(profile.canonical_tolerance) or profile.canonical_tolerance < 0:
        fail(f"invalid canonical tolerance in profile {profile.name}")
    if not math.isfinite(profile.stress_tolerance) or profile.stress_tolerance < 0:
        fail(f"invalid stress tolerance in profile {profile.name}")
    unknown_required = set(profile.required_capabilities) - KNOWN_CAPABILITIES
    if unknown_required:
        fail(
            f"unknown required capability in profile {profile.name}: "
            + ", ".join(sorted(unknown_required))
        )
    for case_name, capability in profile.capability_exclusions:
        if not re.fullmatch(r"[A-Za-z0-9_-]+", case_name):
            fail(f"invalid excluded case name in profile {profile.name}: {case_name}")
        if capability not in KNOWN_CAPABILITIES:
            fail(
                f"unknown capability exclusion in profile {profile.name}: "
                f"{case_name} -> {capability}"
            )
    seen_deviations: set[str] = set()
    for case_name, deviation in profile.documented_deviations:
        if not re.fullmatch(r"[A-Za-z0-9_-]+", case_name):
            fail(f"invalid deviation case in profile {profile.name}: {case_name}")
        if case_name in seen_deviations:
            fail(f"duplicate deviation case in profile {profile.name}: {case_name}")
        seen_deviations.add(case_name)
        if deviation.geometry_ceiling is not None:
            if (
                not math.isfinite(deviation.geometry_ceiling)
                or deviation.geometry_ceiling <= profile.canonical_tolerance
            ):
                fail(
                    f"invalid geometry ceiling in profile {profile.name}: "
                    f"{case_name} -> {deviation.geometry_ceiling}"
                )
        if deviation.geometry_ceiling is None and not deviation.allow_structure:
            fail(f"empty deviation policy in profile {profile.name}: {case_name}")
        if not deviation.note.strip():
            fail(f"undocumented deviation in profile {profile.name}: {case_name}")
    seen_trace_glyph_cases: set[str] = set()
    for case_name, mismatches in profile.canonical_trace_glyph_mismatches:
        if not re.fullmatch(r"[A-Za-z0-9_-]+", case_name):
            fail(
                f"invalid positioned glyph-mismatch case in profile {profile.name}: "
                f"{case_name}"
            )
        if case_name in seen_trace_glyph_cases:
            fail(f"duplicate positioned glyph-mismatch case in profile {profile.name}: {case_name}")
        seen_trace_glyph_cases.add(case_name)
        if not mismatches:
            fail(f"empty positioned glyph-mismatch signature in profile {profile.name}: {case_name}")
        previous_index = -1
        for index, texpose_gid, reference_gid in mismatches:
            if index <= previous_index:
                fail(
                    f"non-increasing positioned glyph paint index in profile {profile.name}: "
                    f"{case_name} -> {index}"
                )
            previous_index = index
            if not (0 <= texpose_gid <= 0xFFFF and 0 <= reference_gid <= 0xFFFF):
                fail(
                    f"out-of-range positioned glyph id in profile {profile.name}: "
                    f"{case_name} -> {texpose_gid}/{reference_gid}"
                )
            if texpose_gid == reference_gid:
                fail(
                    f"non-mismatch positioned glyph signature in profile {profile.name}: "
                    f"{case_name} -> {texpose_gid}/{reference_gid}"
                )
    seen_trace_deviations: set[str] = set()
    for case_name, ceiling in profile.canonical_trace_deviations:
        if not re.fullmatch(r"[A-Za-z0-9_-]+", case_name):
            fail(f"invalid positioned trace deviation in profile {profile.name}: {case_name}")
        if case_name in seen_trace_deviations:
            fail(f"duplicate positioned trace deviation in profile {profile.name}: {case_name}")
        seen_trace_deviations.add(case_name)
        if not math.isfinite(ceiling) or ceiling <= profile.canonical_tolerance:
            fail(
                f"invalid positioned trace ceiling in profile {profile.name}: "
                f"{case_name} -> {ceiling}"
            )
    if profile.reference_environment_sha256 is not None and not re.fullmatch(
        r"[0-9a-f]{64}", profile.reference_environment_sha256
    ):
        fail(f"invalid reference-environment hash in profile {profile.name}")
    for label, value in (
        ("canonical census", profile.canonical_census_sha256),
        ("canonical alias census", profile.canonical_alias_census_sha256),
        ("stress census", profile.stress_census_sha256),
        ("stress alias census", profile.stress_alias_census_sha256),
    ):
        if not re.fullmatch(r"[0-9a-f]{64}", value):
            fail(f"invalid {label} hash in profile {profile.name}")


def resolve_run_spec(args: argparse.Namespace) -> RunSpec:
    if args.font is None:
        name = args.profile or "stix"
        profile = PROFILES[name]
        validate_profile(profile)
        if args.face_index is not None:
            fail("--face-index is owned by a named profile; use --font for an ad-hoc face")
        fixture = ROOT / profile.fixture
        require_file(fixture)
        actual_hash = sha256_file(fixture)
        if actual_hash != profile.sha256:
            fail(
                f"{profile.name}: fixture SHA-256 mismatch: "
                f"expected {profile.sha256}, got {actual_hash}"
            )
        tolerance = profile.stress_tolerance if args.stress else profile.canonical_tolerance
        if args.tolerance is not None:
            if args.fail_on_delta:
                fail("--tolerance cannot override a named profile in a contractual run")
            tolerance = args.tolerance
        return RunSpec(
            name=profile.name,
            font_path=fixture,
            font_sha256=profile.sha256,
            face_index=profile.face_index,
            revision=PROFILE_REVISION,
            measurement_count=(
                profile.stress_measurements if args.stress else profile.canonical_measurements
            ),
            alias_count=profile.stress_aliases if args.stress else profile.canonical_aliases,
            census_sha256=(
                profile.stress_census_sha256
                if args.stress
                else profile.canonical_census_sha256
            ),
            alias_census_sha256=(
                profile.stress_alias_census_sha256
                if args.stress
                else profile.canonical_alias_census_sha256
            ),
            tolerance=tolerance,
            documented_deviations=profile.documented_deviations,
            canonical_trace_glyph_mismatches=(
                profile.canonical_trace_glyph_mismatches if not args.stress else ()
            ),
            canonical_trace_deviations=(
                profile.canonical_trace_deviations if not args.stress else ()
            ),
            reference_environment_sha256=profile.reference_environment_sha256,
            contractual_profile=True,
            collection=fixture.suffix.lower() in {".ttc", ".otc"},
        )

    if args.profile is not None:
        fail("--profile and --font are mutually exclusive")
    font_path = args.font.resolve()
    require_file(font_path)
    face_index = 0 if args.face_index is None else args.face_index
    if not 0 <= face_index <= 0xFFFF_FFFF:
        fail("--face-index must fit u32")
    tolerance = 0.05 if args.tolerance is None else args.tolerance
    return RunSpec(
        name="adhoc",
        font_path=font_path,
        font_sha256=sha256_file(font_path),
        face_index=face_index,
        revision="adhoc-v1",
        measurement_count=89 if args.stress else 21,
        alias_count=94 if args.stress else 21,
        census_sha256=STRESS_CENSUS_SHA256 if args.stress else CANONICAL_CENSUS_SHA256,
        alias_census_sha256=(
            STRESS_ALIAS_CENSUS_SHA256 if args.stress else CANONICAL_ALIAS_CENSUS_SHA256
        ),
        tolerance=tolerance,
        documented_deviations=(),
        canonical_trace_glyph_mismatches=(),
        canonical_trace_deviations=(),
        reference_environment_sha256=None,
        contractual_profile=False,
        collection=font_path.suffix.lower() in {".ttc", ".otc"} or face_index > 0,
    )


def parse_math_probe(
    lines: Iterable[str], spec: RunSpec
) -> tuple[dict[str, dict[str, object]], ProbeMeta]:
    case_re = re.compile(
        r"^TEXPOSE_MATH_COMPARE "
        r"case=(?P<case>[A-Za-z0-9_-]+) "
        r"family=(?P<family>[A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*) "
        r"aliases=(?P<aliases>[A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*) "
        r"style=(?P<style>text|display) "
        r"size_pt=(?P<size>[0-9]+) "
        r"source_utf8_hex=(?P<hex>[0-9a-fA-F]+) "
        r"width_em=(?P<width>[0-9.]+) "
        r"ascent_em=(?P<ascent>[0-9.]+) "
        r"descent_em=(?P<descent>[0-9.]+) "
        r"glyphs=(?P<glyphs>[0-9]+) "
        r"rules=(?P<rules>[0-9]+) "
        r"ops=(?P<ops>[0-9]+) "
        r"trace_primitives=(?P<trace>[0-9]+)$"
    )
    meta_re = re.compile(
        r"^TEXPOSE_MATH_COMPARE_META "
        r"profile=(?P<profile>[A-Za-z0-9_-]+) "
        r"revision=(?P<revision>[A-Za-z0-9_.-]+) "
        r"font_sha256=(?P<hash>[0-9a-f]{64}) "
        r"face_index=(?P<face>[0-9]+)(?: control_glyph_id=[0-9]+)?$"
    )
    glyph_trace_re = re.compile(
        r"^TEXPOSE_MATH_TRACE case=(?P<case>[A-Za-z0-9_-]+) "
        r"index=(?P<index>[0-9]+) kind=glyph glyph_id=(?P<glyph>[0-9]+) "
        r"x=(?P<x>-?[0-9]+/[1-9][0-9]*) "
        r"baseline=(?P<baseline>-?[0-9]+/[1-9][0-9]*) "
        r"scale=(?P<scale>-?[0-9]+/[1-9][0-9]*)$"
    )
    rule_trace_re = re.compile(
        r"^TEXPOSE_MATH_TRACE case=(?P<case>[A-Za-z0-9_-]+) "
        r"index=(?P<index>[0-9]+) kind=rule "
        r"x=(?P<x>-?[0-9]+/[1-9][0-9]*) "
        r"bottom=(?P<bottom>-?[0-9]+/[1-9][0-9]*) "
        r"width=(?P<width>-?[0-9]+/[1-9][0-9]*) "
        r"height=(?P<height>-?[0-9]+/[1-9][0-9]*)$"
    )

    cases: dict[str, dict[str, object]] = {}
    traces: dict[str, list[dict[str, object]]] = {}
    expected: list[str] | None = None
    expected_aliases: list[str] | None = None
    aliases: set[str] = set()
    meta: ProbeMeta | None = None

    for raw in lines:
        line = raw.rstrip("\r\n")

        match = meta_re.match(line)
        if match:
            if meta is not None:
                fail("duplicate math probe metadata")
            group = match.groupdict()
            meta = ProbeMeta(
                profile=group["profile"],
                revision=group["revision"],
                font_sha256=group["hash"],
                face_index=int(group["face"]),
            )
            continue

        match = re.match(
            r"^TEXPOSE_MATH_COMPARE_CASES "
            r"names=([A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*)$",
            line,
        )
        if match:
            if expected is not None:
                fail("duplicate math case census")
            expected = match.group(1).split(",")
            continue

        match = re.match(
            r"^TEXPOSE_MATH_COMPARE_ALIASES "
            r"names=([A-Za-z0-9_-]+(?:,[A-Za-z0-9_-]+)*)$",
            line,
        )
        if match:
            if expected_aliases is not None:
                fail("duplicate math alias census")
            expected_aliases = match.group(1).split(",")
            continue

        match = case_re.match(line)
        if match:
            group = match.groupdict()
            name = group["case"]
            if name in cases:
                fail(f"duplicate math case: {name}")

            case_aliases = group["aliases"].split(",")
            if case_aliases[0] != name:
                fail(f"noncanonical case alias: {name}")
            if any(alias in aliases for alias in case_aliases):
                fail(f"duplicate case alias: {name}")
            aliases.update(case_aliases)

            size = int(group["size"])
            if not 1 <= size <= 4096:
                fail(f"invalid case size: {size}")

            cases[name] = {
                "name": name,
                "family": group["family"],
                "aliases": case_aliases,
                "style": group["style"],
                "size": size,
                "source": utf8hex(group["hex"]),
                "width": parse_float(group["width"]),
                "ascent": parse_float(group["ascent"]),
                "descent": parse_float(group["descent"]),
                "glyphs": parse_int(group["glyphs"], "glyph count"),
                "rules": parse_int(group["rules"], "rule count"),
                "ops": parse_int(group["ops"], "operation count"),
                "trace_primitives": parse_int(group["trace"], "trace primitive count"),
            }
            traces[name] = []
            continue

        match = glyph_trace_re.match(line)
        if match:
            group = match.groupdict()
            name = group["case"]
            if name not in cases:
                fail(f"math probe trace precedes or names unknown case: {name}")
            trace = traces[name]
            index = parse_int(group["index"], "trace paint index")
            if index != len(trace):
                fail(f"nonsequential math probe trace index for {name}: {index}")
            glyph_id = parse_int(group["glyph"], "trace glyph id")
            if glyph_id > 0xFFFF:
                fail(f"trace glyph id exceeds u16 for {name}: {glyph_id}")
            scale = parse_dim_ratio(group["scale"], "glyph scale")
            if scale <= 0:
                fail(f"nonpositive glyph scale in math probe trace for {name}")
            trace.append(
                {
                    "kind": "glyph",
                    "glyph_id": glyph_id,
                    "x": parse_dim_ratio(group["x"], "glyph x"),
                    "baseline": parse_dim_ratio(group["baseline"], "glyph baseline"),
                    "scale": scale,
                }
            )
            continue

        match = rule_trace_re.match(line)
        if match:
            group = match.groupdict()
            name = group["case"]
            if name not in cases:
                fail(f"math probe trace precedes or names unknown case: {name}")
            trace = traces[name]
            index = parse_int(group["index"], "trace paint index")
            if index != len(trace):
                fail(f"nonsequential math probe trace index for {name}: {index}")
            width = parse_dim_ratio(group["width"], "rule width")
            height = parse_dim_ratio(group["height"], "rule height")
            if width <= 0 or height <= 0:
                fail(f"nonpositive rule rectangle in math probe trace for {name}")
            trace.append(
                {
                    "kind": "rule",
                    "x": parse_dim_ratio(group["x"], "rule x"),
                    "bottom": parse_dim_ratio(group["bottom"], "rule bottom"),
                    "width": width,
                    "height": height,
                }
            )
            continue

        if line.startswith("TEXPOSE_MATH_COMPARE") or line.startswith("TEXPOSE_MATH_TRACE"):
            fail(f"malformed math probe record: {line}")

    if meta is None:
        fail("math probe metadata missing")
    if (
        meta.profile != spec.name
        or meta.revision != spec.revision
        or meta.font_sha256 != spec.font_sha256
        or meta.face_index != spec.face_index
    ):
        fail("math probe profile/font identity mismatch")
    if expected is None:
        fail("math probe case census missing")
    if expected_aliases is None:
        fail("math probe alias census missing")
    if len(expected) != len(set(expected)):
        fail("duplicate name in math probe census")
    if set(expected) != set(cases) or len(expected) != len(cases):
        fail("math probe case census missing/incomplete")
    if len(cases) != spec.measurement_count or len(aliases) != spec.alias_count:
        fail(
            f"math profile census changed: {len(cases)} measurements, "
            f"{len(aliases)} aliases"
        )
    actual_census = census_sha256(expected)
    if actual_census != spec.census_sha256:
        fail(
            f"math profile census hash changed: expected {spec.census_sha256}, "
            f"got {actual_census}"
        )
    if len(expected_aliases) != len(set(expected_aliases)):
        fail("duplicate name in math probe alias census")
    if len(expected_aliases) != spec.alias_count or set(expected_aliases) != aliases:
        fail("math probe alias census does not match case records")
    actual_alias_census = census_sha256(expected_aliases)
    if actual_alias_census != spec.alias_census_sha256:
        fail(
            f"math profile alias census hash changed: expected {spec.alias_census_sha256}, "
            f"got {actual_alias_census}"
        )
    for name, case in cases.items():
        trace = traces[name]
        if len(trace) != case["trace_primitives"]:
            fail(
                f"math probe positioned trace is missing/truncated for {name}: "
                f"expected {case['trace_primitives']}, got {len(trace)}"
            )
        case["trace"] = trace

    return cases, meta

def build_math_tex(
    cases: dict[str, dict[str, object]],
    result: str,
    font_name: str,
    spec: RunSpec,
) -> str:
    mathfont_options = ["Path=./"]
    if spec.collection:
        mathfont_options.append(f"FontIndex={spec.face_index}")
    options = ",".join(mathfont_options)
    out = [
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\usepackage[mathrm=sym,mathbf=sym,mathit=sym,mathsf=sym,mathtt=sym]{unicode-math}",
        rf"\setmathfont{{{font_name}}}[{options}]",
        r"\setoperatorfont\symrm",
        r"\pagestyle{empty}",
        r"\mathsurround=0pt",
        rf"\directlua{{texpose_math_result_file='{result}'; dofile('math_compare.lua')}}",
        r"\edef\TexposeUnicodeMathVersion{\csname ver@unicode-math.sty\endcsname}",
        r"\edef\TexposeFontspecVersion{\csname ver@fontspec.sty\endcsname}",
        r"\edef\TexposeAmsmathVersion{\csname ver@amsmath.sty\endcsname}",
        r"\begin{document}",
        r"\makeatletter",
        r"\@ifpackageloaded{microtype}{\errmessage{TeXpose math oracle forbids microtype}}{}",
        r"\makeatother",
        r"\directlua{texpose_write_fingerprint("
        r'"\luaescapestring{\fmtversion}",',
        r'"\luaescapestring{\TexposeUnicodeMathVersion}",',
        r'"\luaescapestring{\TexposeFontspecVersion}",',
        r'"\luaescapestring{\TexposeAmsmathVersion}",',
        rf'"{spec.font_sha256}",{spec.face_index},"{spec.name}",',
        rf'"{spec.revision}","{spec.census_sha256}","{spec.alias_census_sha256}")}}',
    ]

    for case in cases.values():
        name = str(case["name"])
        source = str(case["source"])
        if re.search(r"[\r\n%]", source) or not re.fullmatch(r"[A-Za-z0-9_-]+", name):
            fail(f"unsafe math fixture: {name}")

        style = r"\displaystyle" if case["style"] == "display" else r"\textstyle"
        size = int(case["size"])
        out += [
            r"\begingroup",
            rf"\fontsize{{{size}pt}}{{{size}pt}}\selectfont",
            rf"\setbox0=\hbox{{$" + style + " " + source + "$}",
            r"\setbox2=\hbox{$\textstyle x$}",
            r"\setbox4=\hbox{$\scriptstyle x$}",
            r"\setbox6=\hbox{$\scriptscriptstyle x$}",
            rf"\directlua{{texpose_measure_math_case('{name}', 0, 2, 4, 6)}}",
            r"\endgroup",
        ]

    out.append(r"\end{document}")
    return "\n".join(out) + "\n"


def parse_math_results(
    path: Path,
    cases: dict[str, dict[str, object]],
    spec: RunSpec,
) -> tuple[dict[str, dict[str, object]], ReferenceFingerprint]:
    require_file(path)
    result: dict[str, dict[str, object]] = {}
    fingerprint: ReferenceFingerprint | None = None

    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split("|")
        if parts[0] == "FINGERPRINT":
            if len(parts) != 13 or fingerprint is not None:
                fail(f"unexpected/malformed LuaTeX fingerprint: {line}")
            engine = utf8hex(parts[1])
            distribution = utf8hex(parts[2])
            if not distribution or distribution not in engine:
                fail(
                    "LuaTeX banner does not identify the recorded TeX distribution: "
                    f"{engine}"
                )
            fingerprint = ReferenceFingerprint(
                engine=engine,
                distribution=distribution,
                latex=utf8hex(parts[3]),
                unicode_math=utf8hex(parts[4]),
                fontspec=utf8hex(parts[5]),
                amsmath=utf8hex(parts[6]),
                font_sha256=parts[7],
                face_index=parse_int(parts[8], "fingerprint face index"),
                profile=parts[9],
                revision=parts[10],
                census_sha256=parts[11],
                alias_census_sha256=parts[12],
            )
            continue

        if parts[0] == "CASE":
            if len(parts) != 15 or parts[1] not in cases:
                fail(f"unexpected/malformed LuaTeX math result: {line}")
            name = parts[1]
            if name in result:
                fail(f"duplicate LuaTeX math case: {name}")

            text_em = parse_float(parts[5])
            script_em = parse_float(parts[6])
            scriptscript_em = parse_float(parts[7])
            if not (text_em > script_em >= scriptscript_em > 0):
                fail(
                    f"LuaTeX style-size ordering is invalid for {name}: "
                    f"{text_em}, {script_em}, {scriptscript_em}"
                )

            size = int(cases[name]["size"])
            if abs(text_em - size * 65536) > 2:
                fail(f"LuaTeX selected wrong root math size for {name}")

            trace_primitives = parse_int(parts[13], "trace primitive count")
            control_subfont = parse_int(parts[14], "control subfont")
            if spec.collection and control_subfont != spec.face_index + 1:
                fail(
                    f"LuaTeX selected wrong collection face for {name}: "
                    f"subfont {control_subfont}, expected {spec.face_index + 1}"
                )

            result[name] = {
                "width": parse_float(parts[2]) / text_em,
                "ascent": parse_float(parts[3]) / text_em,
                "descent": parse_float(parts[4]) / text_em,
                "text_em": text_em,
                "script_em": script_em,
                "scriptscript_em": scriptscript_em,
                "glyphs": parse_int(parts[8], "glyph count"),
                "rules": parse_int(parts[9], "rule count"),
                "hlists": parse_int(parts[10], "hlist count"),
                "vlists": parse_int(parts[11], "vlist count"),
                "max_depth": parse_int(parts[12], "max depth"),
                "trace_primitives": trace_primitives,
                "control_subfont": control_subfont,
                "trace": [],
            }
            continue

        if parts[0] == "TRACE":
            if len(parts) != 8 or parts[1] not in result:
                fail(f"unexpected/malformed LuaTeX positioned trace: {line}")
            name = parts[1]
            trace = result[name]["trace"]
            if not isinstance(trace, list):
                fail(f"internal LuaTeX trace container is invalid for {name}")
            index = parse_int(parts[2], "trace paint index")
            if index != len(trace):
                fail(f"nonsequential LuaTeX trace index for {name}: {index}")
            kind = parts[3]
            if kind == "G":
                glyph_id = parse_int(parts[4], "trace glyph id")
                if glyph_id > 0xFFFF:
                    fail(f"LuaTeX trace glyph id exceeds u16 for {name}: {glyph_id}")
                x = parse_signed_int(parts[5], "trace glyph x")
                baseline = parse_signed_int(parts[6], "trace glyph baseline")
                font_size = parse_int(parts[7], "trace glyph font size")
                if font_size == 0 or font_size > MAX_TEX_DIM:
                    fail(f"invalid LuaTeX trace glyph font size for {name}: {font_size}")
                trace.append(
                    {
                        "kind": "glyph",
                        "glyph_id": glyph_id,
                        "x_sp": x,
                        "baseline_sp": baseline,
                        "font_size_sp": font_size,
                    }
                )
            elif kind == "R":
                x = parse_signed_int(parts[4], "trace rule x")
                bottom = parse_signed_int(parts[5], "trace rule bottom")
                width = parse_int(parts[6], "trace rule width")
                height = parse_int(parts[7], "trace rule height")
                if width == 0 or height == 0 or width > MAX_TEX_DIM or height > MAX_TEX_DIM:
                    fail(f"invalid LuaTeX rule rectangle for {name}")
                trace.append(
                    {
                        "kind": "rule",
                        "x_sp": x,
                        "bottom_sp": bottom,
                        "width_sp": width,
                        "height_sp": height,
                    }
                )
            else:
                fail(f"unknown LuaTeX positioned trace primitive kind for {name}: {kind}")
            continue

        fail(f"unexpected/malformed LuaTeX math result: {line}")

    if fingerprint is None:
        fail("LuaTeX reference fingerprint missing")
    if len(result) != len(cases):
        fail(f"expected {len(cases)} LuaTeX cases, got {len(result)}")
    if (
        fingerprint.font_sha256 != spec.font_sha256
        or fingerprint.face_index != spec.face_index
        or fingerprint.profile != spec.name
        or fingerprint.revision != spec.revision
        or fingerprint.census_sha256 != spec.census_sha256
        or fingerprint.alias_census_sha256 != spec.alias_census_sha256
    ):
        fail("LuaTeX reference profile/font identity mismatch")
    for label, value in (
        ("LaTeX format", fingerprint.latex),
        ("unicode-math", fingerprint.unicode_math),
        ("fontspec", fingerprint.fontspec),
        ("amsmath", fingerprint.amsmath),
    ):
        if not value.strip():
            fail(f"empty reference fingerprint field: {label}")
    for name, row in result.items():
        trace = row["trace"]
        if not isinstance(trace, list):
            fail(f"internal LuaTeX trace container is invalid for {name}")
        if len(trace) != row["trace_primitives"]:
            fail(
                f"LuaTeX positioned trace is missing/truncated for {name}: "
                f"expected {row['trace_primitives']}, got {len(trace)}"
            )

    return result, fingerprint

def run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
) -> list[str]:
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)

    process = subprocess.run(
        command,
        cwd=cwd,
        env=merged_env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    lines = process.stdout.splitlines()
    if process.returncode != 0:
        tail = "\n".join(lines[-80:])
        fail(f"command failed ({process.returncode}): {' '.join(command)}\n{tail}")
    return lines


def percentile(values: list[float], percent: int) -> float:
    if not values:
        return 0.0
    index = math.ceil(percent / 100 * len(values)) - 1
    return values[max(0, min(len(values) - 1, index))]


def print_maximum(label: str, deltas: Iterable[CaseDelta]) -> None:
    values = list(deltas)
    if not values:
        return
    item = max(values, key=lambda delta: delta.maximum)
    print(
        f"  {label}: {item.name} | {item.maximum_dimension} "
        f"{item.maximum:.6f}em"
    )


def print_aggregate_diagnostics(spec: RunSpec, deltas: list[CaseDelta]) -> None:
    print("maxima:")
    print_maximum(f"font {spec.name}", deltas)
    families = sorted(
        {family for delta in deltas for family in delta.family.split(",")}
    )
    for family in families:
        print_maximum(
            f"family {family}",
            (d for d in deltas if family in d.family.split(",")),
        )
    sizes = sorted({delta.size for delta in deltas})
    for size in sizes:
        print_maximum(f"size {size}pt", (d for d in deltas if d.size == size))
    for dimension in ("width", "ascent", "descent"):
        item = max(deltas, key=lambda delta: getattr(delta, dimension))
        print(f"  dimension {dimension}: {item.name} | {getattr(item, dimension):.6f}em")


def compare_positioned_trace(
    case: dict[str, object], reference: dict[str, object]
) -> TraceCaseDelta:
    name = str(case["name"])
    family = str(case["family"])
    size = int(case["size"])
    actual = case["trace"]
    expected = reference["trace"]
    if not isinstance(actual, list) or not isinstance(expected, list):
        fail(f"internal positioned trace container is invalid for {name}")

    if len(actual) != len(expected):
        return TraceCaseDelta(
            name=name,
            family=family,
            size=size,
            primitive_count=min(len(actual), len(expected)),
            topology_mismatch=f"primitive count {len(actual)}/{len(expected)}",
            glyph_mismatches=(),
            glyph_multiset_matches=None,
            geometry_alignment=None,
            maximum=0.0,
            maximum_field=None,
            maximum_index=None,
            maximum_reference_index=None,
        )

    for index, (left, right) in enumerate(zip(actual, expected, strict=True)):
        if left["kind"] != right["kind"]:
            return TraceCaseDelta(
                name=name,
                family=family,
                size=size,
                primitive_count=len(actual),
                topology_mismatch=(
                    f"primitive {index} kind {left['kind']}/{right['kind']}"
                ),
                glyph_mismatches=(),
                glyph_multiset_matches=None,
                geometry_alignment=None,
                maximum=0.0,
                maximum_field=None,
                maximum_index=None,
                maximum_reference_index=None,
            )

    glyph_mismatches = tuple(
        GlyphMismatch(
            index=index,
            texpose_glyph_id=int(left["glyph_id"]),
            reference_glyph_id=int(right["glyph_id"]),
        )
        for index, (left, right) in enumerate(zip(actual, expected, strict=True))
        if left["kind"] == "glyph" and left["glyph_id"] != right["glyph_id"]
    )
    actual_glyph_ids = [
        int(primitive["glyph_id"])
        for primitive in actual
        if primitive["kind"] == "glyph"
    ]
    expected_glyph_ids = [
        int(primitive["glyph_id"])
        for primitive in expected
        if primitive["kind"] == "glyph"
    ]
    glyph_multiset_matches = sorted(actual_glyph_ids) == sorted(expected_glyph_ids)
    all_primitives_are_glyphs = len(actual_glyph_ids) == len(actual)

    geometry_alignment: str | None
    primitive_pairs: list[tuple[int, dict[str, object], int, dict[str, object]]] = []
    if not glyph_mismatches:
        geometry_alignment = "paint-index"
        primitive_pairs = [
            (index, left, index, right)
            for index, (left, right) in enumerate(zip(actual, expected, strict=True))
        ]
    elif (
        glyph_multiset_matches
        and all_primitives_are_glyphs
        and len(set(actual_glyph_ids)) == len(actual_glyph_ids)
    ):
        # For a glyph-only pure reorder, unique glyph IDs are an independent
        # identity key. Realign glyph geometry by ID, but keep the paint-order
        # mismatch itself as contractual evidence. Never use coordinate proximity
        # to choose a match.
        geometry_alignment = "glyph-id"
        expected_by_glyph = {
            int(primitive["glyph_id"]): (index, primitive)
            for index, primitive in enumerate(expected)
        }
        primitive_pairs = [
            (
                index,
                primitive,
                expected_by_glyph[int(primitive["glyph_id"])][0],
                expected_by_glyph[int(primitive["glyph_id"])][1],
            )
            for index, primitive in enumerate(actual)
        ]
    else:
        # Selection changes have no identity-preserving match. Duplicate glyph IDs
        # cannot be paired uniquely, and a reordered mixed glyph/rule trace does
        # not prove rule identity by paint index. Preserve the mismatch evidence
        # and withhold numeric geometry instead of inventing a correspondence.
        geometry_alignment = None

    text_em = float(reference["text_em"])
    maximum = 0.0
    maximum_field: str | None = None
    maximum_index: int | None = None
    maximum_reference_index: int | None = None

    for index, left, reference_index, right in primitive_pairs:
        if left["kind"] == "glyph":
            values = (
                ("glyph-x", float(left["x"]), float(right["x_sp"]) / text_em),
                (
                    "glyph-baseline",
                    float(left["baseline"]),
                    float(right["baseline_sp"]) / text_em,
                ),
                (
                    "glyph-scale",
                    float(left["scale"]),
                    float(right["font_size_sp"]) / text_em,
                ),
            )
        else:
            values = (
                ("rule-x", float(left["x"]), float(right["x_sp"]) / text_em),
                (
                    "rule-bottom",
                    float(left["bottom"]),
                    float(right["bottom_sp"]) / text_em,
                ),
                (
                    "rule-width",
                    float(left["width"]),
                    float(right["width_sp"]) / text_em,
                ),
                (
                    "rule-height",
                    float(left["height"]),
                    float(right["height_sp"]) / text_em,
                ),
            )
        for field, left_value, right_value in values:
            delta = abs(left_value - right_value)
            if delta > maximum:
                maximum = delta
                maximum_field = field
                maximum_index = index
                maximum_reference_index = reference_index

    return TraceCaseDelta(
        name=name,
        family=family,
        size=size,
        primitive_count=len(actual),
        topology_mismatch=None,
        glyph_mismatches=glyph_mismatches,
        glyph_multiset_matches=glyph_multiset_matches,
        geometry_alignment=geometry_alignment,
        maximum=maximum,
        maximum_field=maximum_field,
        maximum_index=maximum_index,
        maximum_reference_index=maximum_reference_index,
    )

def print_positioned_trace_diagnostics(
    trace_deltas: list[TraceCaseDelta], top_worst: int
) -> None:
    topology_matches = [delta for delta in trace_deltas if delta.topology_matches]
    identity_aligned = [delta for delta in topology_matches if delta.glyphs_match]
    reorder_cases = [
        delta
        for delta in topology_matches
        if delta.glyph_mismatches and delta.glyph_multiset_matches
    ]
    reorder_realigned = [
        delta for delta in reorder_cases if delta.geometry_alignment == "glyph-id"
    ]
    geometry_comparable = [delta for delta in topology_matches if delta.geometry_comparable]
    geometry_values = sorted(delta.maximum for delta in geometry_comparable)
    print(
        "positioned trace: "
        f"kind topology {len(topology_matches)}/{len(trace_deltas)} | "
        f"glyph identity/order aligned {len(identity_aligned)}/{len(topology_matches)} | "
        f"reorder glyph geometry realigned {len(reorder_realigned)}/{len(reorder_cases)} | "
        f"geometry comparable {len(geometry_comparable)}/{len(topology_matches)} | "
        f"p50 {percentile(geometry_values, 50):.6f} | "
        f"p90 {percentile(geometry_values, 90):.6f} | "
        f"p95 {percentile(geometry_values, 95):.6f} | "
        f"p99 {percentile(geometry_values, 99):.6f} | "
        f"max {max(geometry_values, default=0.0):.6f}"
    )

    topology_mismatches = [
        delta for delta in trace_deltas if delta.topology_mismatch is not None
    ]
    if topology_mismatches:
        print("positioned kind-topology mismatches:", file=sys.stderr)
        for delta in topology_mismatches:
            print(f"  {delta.name}: {delta.topology_mismatch}", file=sys.stderr)

    glyph_mismatches = [delta for delta in trace_deltas if delta.glyph_mismatches]
    if glyph_mismatches:
        print("positioned glyph/order mismatches:", file=sys.stderr)
        for delta in glyph_mismatches:
            if not delta.glyph_multiset_matches:
                relation = "glyph selection differs; geometry not comparable"
            elif delta.geometry_alignment == "glyph-id":
                relation = (
                    "same glyph multiset, different paint order; "
                    "glyph geometry realigned by id"
                )
            else:
                relation = (
                    "same glyph multiset, different paint order; "
                    "geometry realignment ambiguous"
                )
            print(f"  {delta.name}: {relation}", file=sys.stderr)
            for mismatch in delta.glyph_mismatches:
                print(
                    f"    primitive {mismatch.index} glyph "
                    f"{mismatch.texpose_glyph_id}/{mismatch.reference_glyph_id}",
                    file=sys.stderr,
                )

    if geometry_comparable:
        print("worst positioned deltas (identity-realigned):", file=sys.stderr)
        for delta in sorted(
            geometry_comparable, key=lambda item: item.maximum, reverse=True
        )[:top_worst]:
            if delta.maximum_field is None:
                location = "none"
            elif delta.maximum_index == delta.maximum_reference_index:
                location = f"primitive {delta.maximum_index} {delta.maximum_field}"
            else:
                location = (
                    f"TeXpose primitive {delta.maximum_index}/reference primitive "
                    f"{delta.maximum_reference_index} {delta.maximum_field}"
                )
            print(
                f"  {delta.name} | {delta.family} | {delta.size}pt | "
                f"{location} | {delta.maximum:.6f}em",
                file=sys.stderr,
            )


def validate_canonical_positioned_contract(
    spec: RunSpec, trace_deltas: list[TraceCaseDelta]
) -> list[str]:
    topology_mismatches = [
        delta.name for delta in trace_deltas if not delta.topology_matches
    ]
    if topology_mismatches:
        fail(
            "canonical positioned topology mismatches: "
            + ", ".join(topology_mismatches)
        )

    expected_glyph_signatures = dict(spec.canonical_trace_glyph_mismatches)
    observed_glyph_signatures = {
        delta.name: tuple(
            (mismatch.index, mismatch.texpose_glyph_id, mismatch.reference_glyph_id)
            for mismatch in delta.glyph_mismatches
        )
        for delta in trace_deltas
        if delta.glyph_mismatches
    }
    expected_glyph_cases = set(expected_glyph_signatures)
    observed_glyph_cases = set(observed_glyph_signatures)
    new_glyph_cases = observed_glyph_cases - expected_glyph_cases
    repaired_glyph_cases = expected_glyph_cases - observed_glyph_cases
    changed_glyph_cases = {
        case_name
        for case_name in expected_glyph_cases & observed_glyph_cases
        if expected_glyph_signatures[case_name] != observed_glyph_signatures[case_name]
    }
    if new_glyph_cases or repaired_glyph_cases or changed_glyph_cases:
        pieces = []
        if new_glyph_cases:
            pieces.append(
                "new positioned glyph/order mismatch cases: "
                + ", ".join(sorted(new_glyph_cases))
            )
        if repaired_glyph_cases:
            pieces.append(
                "documented positioned glyph/order mismatch cases are no longer observed: "
                + ", ".join(sorted(repaired_glyph_cases))
            )
        if changed_glyph_cases:
            pieces.append(
                "positioned glyph/order mismatch signatures changed: "
                + ", ".join(sorted(changed_glyph_cases))
            )
        fail("; ".join(pieces))

    ceilings = dict(spec.canonical_trace_deviations)
    unknown_ceilings = set(ceilings) - {delta.name for delta in trace_deltas}
    if unknown_ceilings:
        fail(
            "profile documents unknown positioned trace cases: "
            + ", ".join(sorted(unknown_ceilings))
        )

    approved: list[str] = []
    unapproved: list[str] = []
    observed_ceiling_cases: set[str] = set()
    for delta in trace_deltas:
        # Selection mismatches have no identity-preserving geometry. Pure glyph-only
        # paint-order differences are realigned by unique glyph ID and become
        # contractual geometry evidence. A ceiling may cover such a realigned reorder;
        # non-comparable mismatches cannot consume it and therefore fail as stale.
        if not delta.geometry_comparable or delta.maximum <= spec.tolerance:
            continue
        ceiling = ceilings.get(delta.name)
        if ceiling is not None and delta.maximum <= ceiling:
            approved.append(delta.name)
            observed_ceiling_cases.add(delta.name)
        else:
            unapproved.append(delta.name)

    stale_ceilings = set(ceilings) - observed_ceiling_cases
    if stale_ceilings:
        fail(
            "documented positioned trace deviations are no longer observed: "
            + ", ".join(sorted(stale_ceilings))
        )
    if unapproved:
        fail(
            f"positioned geometry exceeded {spec.tolerance}em without a bounded "
            "trace deviation: " + ", ".join(unapproved)
        )
    return approved

def geometry_deviation_is_approved(
    delta: float, tolerance: float, deviation: Deviation | None
) -> bool:
    if delta <= tolerance:
        return True
    return (
        deviation is not None
        and deviation.geometry_ceiling is not None
        and delta <= deviation.geometry_ceiling
    )


def validate_reference_environment(
    spec: RunSpec, fingerprint: ReferenceFingerprint
) -> str:
    environment_sha256 = fingerprint.environment_sha256()
    if (
        spec.reference_environment_sha256 is not None
        and environment_sha256 != spec.reference_environment_sha256
    ):
        fail(
            f"unknown reference fingerprint for profile {spec.name}: "
            f"expected {spec.reference_environment_sha256}, got {environment_sha256}"
        )
    return environment_sha256


def gate_math(
    *,
    args: argparse.Namespace,
) -> None:
    if args.tolerance is not None and (
        not math.isfinite(args.tolerance) or args.tolerance < 0
    ):
        fail("tolerance must be finite/nonnegative")
    if not 1 <= args.top_worst <= 100:
        fail("--top-worst must be 1..100")

    require_file(ROOT / "Cargo.toml")
    require_file(ROOT / "tools" / "math_compare.lua")
    spec = resolve_run_spec(args)

    cargo = shutil.which("cargo")
    lualatex = shutil.which("lualatex")
    if cargo is None:
        fail("cargo is not available on PATH")
    if lualatex is None:
        fail("lualatex is not available on PATH")

    with tempfile.TemporaryDirectory(prefix="texpose-math-") as raw_work:
        work = Path(raw_work)
        shutil.copy2(ROOT / "tools" / "math_compare.lua", work / "math_compare.lua")
        suffix = spec.font_path.suffix.lower()
        if spec.collection:
            safe_suffix = suffix if suffix in {".ttc", ".otc"} else ".ttc"
        else:
            safe_suffix = ".ttf" if suffix == ".ttf" else ".otf"
        font = work / f"oracle-font{safe_suffix}"
        shutil.copy2(spec.font_path, font)
        if sha256_file(font) != spec.font_sha256:
            fail("copied oracle font SHA-256 changed")
        target = work / "target"

        env = {
            "CARGO_TARGET_DIR": str(target),
            "TEXPOSE_MATH_COMPARE_FONT": str(font),
            "TEXPOSE_MATH_COMPARE_FONT_SHA256": spec.font_sha256,
            "TEXPOSE_MATH_COMPARE_FACE_INDEX": str(spec.face_index),
            "TEXPOSE_MATH_COMPARE_PROFILE": spec.name,
            "TEXPOSE_MATH_COMPARE_REVISION": spec.revision,
        }
        if args.stress:
            env["TEXPOSE_MATH_COMPARE_STRESS"] = "1"

        print(f"Math oracle [{spec.name}]: TeXpose probe", file=sys.stderr)
        lines = run(
            [
                cargo,
                "test",
                "--manifest-path",
                str(ROOT / "Cargo.toml"),
                "--test",
                "math_oracle",
                "--release",
                "lualatex_math_comparison_probe",
                "--",
                "--exact",
                "--ignored",
                "--nocapture",
            ],
            cwd=ROOT,
            env=env,
        )

        cases, probe_meta = parse_math_probe(lines, spec)
        tex = work / "math-compare.tex"
        result = work / "math-compare.tsv"
        tex.write_text(
            build_math_tex(cases, result.name, font.name, spec),
            encoding="utf-8",
        )

        print(f"Math oracle [{spec.name}]: LuaLaTeX reference", file=sys.stderr)
        run(
            [
                lualatex,
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-file-line-error",
                tex.name,
            ],
            cwd=work,
        )
        reference, fingerprint = parse_math_results(result, cases, spec)

        environment_sha256 = validate_reference_environment(spec, fingerprint)
        print(
            "reference fingerprint: "
            f"engine={fingerprint.engine!r} | distribution={fingerprint.distribution!r} | "
            f"latex={fingerprint.latex!r} | unicode-math={fingerprint.unicode_math!r} | "
            f"fontspec={fingerprint.fontspec!r} | amsmath={fingerprint.amsmath!r} | "
            f"sha256={environment_sha256}",
            file=sys.stderr,
        )
        if spec.reference_environment_sha256 is None and spec.contractual_profile:
            print(
                f"reference fingerprint for profile {spec.name} is not pinned yet",
                file=sys.stderr,
            )

        deviations = dict(spec.documented_deviations)
        unknown_deviations = set(deviations) - set(cases)
        if unknown_deviations:
            fail(
                f"profile {spec.name} documents unknown cases: "
                + ", ".join(sorted(unknown_deviations))
            )
        deltas: list[CaseDelta] = []
        over_tolerance: list[str] = []
        approved_geometry: list[str] = []
        unapproved_geometry: list[str] = []
        structural: list[str] = []
        approved_structural: list[str] = []
        unapproved_structural: list[str] = []
        observed_deviation_cases: set[str] = set()
        trace_deltas: list[TraceCaseDelta] = []

        for case in cases.values():
            name = str(case["name"])
            expected = reference[name]
            delta = CaseDelta(
                name=name,
                family=str(case["family"]),
                size=int(case["size"]),
                width=abs(float(case["width"]) - float(expected["width"])),
                ascent=abs(float(case["ascent"]) - float(expected["ascent"])),
                descent=abs(float(case["descent"]) - float(expected["descent"])),
                texpose_glyphs=int(case["glyphs"]),
                reference_glyphs=int(expected["glyphs"]),
                texpose_rules=int(case["rules"]),
                reference_rules=int(expected["rules"]),
            )
            deltas.append(delta)
            trace_deltas.append(compare_positioned_trace(case, expected))
            deviation = deviations.get(name)
            if deviation is not None and (
                delta.maximum > spec.tolerance or delta.structural_mismatch
            ):
                observed_deviation_cases.add(name)
            if delta.maximum > spec.tolerance:
                over_tolerance.append(name)
                if geometry_deviation_is_approved(
                    delta.maximum, spec.tolerance, deviation
                ):
                    approved_geometry.append(name)
                else:
                    unapproved_geometry.append(name)
            if delta.structural_mismatch:
                structural.append(name)
                if deviation is not None and deviation.allow_structure:
                    approved_structural.append(name)
                else:
                    unapproved_structural.append(name)

        values = sorted(delta.maximum for delta in deltas)
        print(
            f"{len(cases) - len(over_tolerance)}/{len(cases)} "
            f"<={spec.tolerance:.3f}em | "
            f"approved geometry deviations {len(approved_geometry)} | "
            f"p50 {percentile(values, 50):.6f} | "
            f"p90 {percentile(values, 90):.6f} | "
            f"p95 {percentile(values, 95):.6f} | "
            f"p99 {percentile(values, 99):.6f} | "
            f"max {max(values):.6f}"
        )
        print_aggregate_diagnostics(spec, deltas)
        print_positioned_trace_diagnostics(trace_deltas, args.top_worst)

        if approved_geometry:
            print("bounded geometry deviations:", file=sys.stderr)
            delta_by_name = {delta.name: delta for delta in deltas}
            for name in approved_geometry:
                deviation = deviations[name]
                print(
                    f"  {name}: {delta_by_name[name].maximum:.6f}em "
                    f"<= {deviation.geometry_ceiling:.6f}em | {deviation.note}",
                    file=sys.stderr,
                )
        if structural:
            print("structure mismatches: " + ", ".join(structural), file=sys.stderr)
        if approved_structural:
            print(
                "approved structure deviations: " + ", ".join(approved_structural),
                file=sys.stderr,
            )

        print("worst cases:", file=sys.stderr)
        for delta in sorted(deltas, key=lambda item: item.maximum, reverse=True)[
            : args.top_worst
        ]:
            print(
                f"  {delta.name} | {delta.family} | {delta.size}pt | "
                f"width {delta.width:.6f} | ascent {delta.ascent:.6f} | "
                f"descent {delta.descent:.6f} | "
                f"glyphs {delta.texpose_glyphs}/{delta.reference_glyphs} | "
                f"rules {delta.texpose_rules}/{delta.reference_rules}",
                file=sys.stderr,
            )

        if args.fail_on_delta and spec.contractual_profile and (
            spec.reference_environment_sha256 is None
        ):
            fail(
                f"profile {spec.name} has no pinned reference-environment identity; "
                "record measurement evidence before accepting a contractual run"
            )
        if args.fail_on_delta and spec.contractual_profile and not args.stress:
            stale_deviations = set(deviations) - observed_deviation_cases
            if stale_deviations:
                fail(
                    "documented deviations are no longer observed: "
                    + ", ".join(sorted(stale_deviations))
                )
        if args.fail_on_delta and (unapproved_geometry or unapproved_structural):
            pieces = []
            if unapproved_geometry:
                pieces.append(
                    f"geometry exceeded {spec.tolerance}em without a bounded deviation: "
                    + ", ".join(unapproved_geometry)
                )
            if unapproved_structural:
                pieces.append("structure mismatches: " + ", ".join(unapproved_structural))
            fail("; ".join(pieces))

        if args.fail_on_delta and spec.contractual_profile and not args.stress:
            approved_trace = validate_canonical_positioned_contract(spec, trace_deltas)
            if approved_trace:
                ceilings = dict(spec.canonical_trace_deviations)
                by_name = {delta.name: delta for delta in trace_deltas}
                print("bounded positioned trace deviations:", file=sys.stderr)
                for name in approved_trace:
                    print(
                        f"  {name}: {by_name[name].maximum:.6f}em "
                        f"<= {ceilings[name]:.6f}em",
                        file=sys.stderr,
                    )


def self_test() -> None:
    for profile in PROFILES.values():
        validate_profile(profile)

    bounded = Deviation(
        geometry_ceiling=0.0811,
        allow_structure=False,
        note="self-test bounded geometry deviation",
    )
    if not geometry_deviation_is_approved(0.0810, 0.05, bounded):
        fail("self-test bounded geometry deviation rejected")
    if geometry_deviation_is_approved(0.0812, 0.05, bounded):
        fail("self-test geometry ceiling failed open")

    names = [f"case-{index}" for index in range(1, 22)]
    census = "TEXPOSE_MATH_COMPARE_CASES names=" + ",".join(names)
    alias_census = "TEXPOSE_MATH_COMPARE_ALIASES names=" + ",".join(names)
    meta = (
        "TEXPOSE_MATH_COMPARE_META profile=test revision=test-v1 "
        f"font_sha256={'a' * 64} face_index=0"
    )

    def probe_case(name: str, *, trace_count: int = 1) -> str:
        return (
            "TEXPOSE_MATH_COMPARE "
            f"case={name} family=basic aliases={name} style=text "
            "size_pt=10 source_utf8_hex=78 width_em=1.0 "
            "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1 "
            f"trace_primitives={trace_count}"
        )

    def probe_trace(name: str, *, index: int = 0) -> str:
        return (
            f"TEXPOSE_MATH_TRACE case={name} index={index} kind=glyph glyph_id=42 "
            "x=0/1 baseline=0/1 scale=1/1"
        )

    rows: list[str] = []
    for name in names:
        rows.extend((probe_case(name), probe_trace(name)))

    spec = RunSpec(
        name="test",
        font_path=Path("unused.otf"),
        font_sha256="a" * 64,
        face_index=0,
        revision="test-v1",
        measurement_count=21,
        alias_count=21,
        census_sha256=census_sha256(names),
        alias_census_sha256=census_sha256(names),
        tolerance=0.05,
        documented_deviations=(),
        canonical_trace_glyph_mismatches=(),
        canonical_trace_deviations=(),
        reference_environment_sha256=None,
        contractual_profile=False,
        collection=False,
    )

    oracle_source = build_math_tex(
        {
            "case-1": {
                "name": "case-1",
                "source": "x",
                "style": "text",
                "size": 10,
            }
        },
        "result.tsv",
        "STIXTwoMath-Regular.otf",
        spec,
    )
    if r"\usepackage{microtype}" in oracle_source:
        fail("self-test math oracle unexpectedly loads microtype")
    guard_marker = r"\@ifpackageloaded{microtype}"
    if guard_marker not in oracle_source:
        fail("self-test math oracle lost the microtype exclusion guard")
    begin_index = oracle_source.index(r"\begin{document}")
    guard_index = oracle_source.index(guard_marker)
    fingerprint_index = oracle_source.index(r"\directlua{texpose_write_fingerprint(")
    if not begin_index < guard_index < fingerprint_index:
        fail("self-test microtype guard must run after begin-document hooks and before evidence")

    cases, parsed_meta = parse_math_probe([meta, census, alias_census, *rows], spec)
    if len(cases) != 21 or parsed_meta.face_index != 0:
        fail("self-test valid probe rejected")

    grouped_names = ["group-a", "single", "group-b"]
    grouped_spec = RunSpec(
        name="test",
        font_path=Path("unused.otf"),
        font_sha256="a" * 64,
        face_index=0,
        revision="test-v1",
        measurement_count=2,
        alias_count=3,
        census_sha256=census_sha256(["group-a", "single"]),
        alias_census_sha256=census_sha256(grouped_names),
        tolerance=0.05,
        documented_deviations=(),
        canonical_trace_glyph_mismatches=(),
        canonical_trace_deviations=(),
        reference_environment_sha256=None,
        contractual_profile=False,
        collection=False,
    )
    grouped_probe = [
        meta,
        "TEXPOSE_MATH_COMPARE_CASES names=group-a,single",
        "TEXPOSE_MATH_COMPARE_ALIASES names=group-a,single,group-b",
        "TEXPOSE_MATH_COMPARE case=group-a family=basic aliases=group-a,group-b "
        "style=text size_pt=10 source_utf8_hex=78 width_em=1.0 "
        "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1 trace_primitives=1",
        probe_trace("group-a"),
        probe_case("single"),
        probe_trace("single"),
    ]
    parse_math_probe(grouped_probe, grouped_spec)

    bad_probe_sets = (
        [census, alias_census, *rows],
        [meta, census, alias_census, *rows[:-2]],
        [meta, census, alias_census, *rows, probe_case(names[0]), probe_trace(names[0])],
        [meta, *rows],
        [meta, census, alias_census, *rows, "TEXPOSE_MATH_COMPARE broken"],
        [meta, census, alias_census, probe_case(names[0]), *rows[2:]],
        [meta, census, alias_census, probe_case(names[0]), probe_trace(names[0], index=1), *rows[2:]],
        [
            meta,
            census,
            alias_census,
            probe_case(names[0]),
            probe_trace(names[0]).replace("x=0/1", "x=2/2"),
            *rows[2:],
        ],
        [
            meta,
            census,
            alias_census,
            probe_case(names[0]),
            probe_trace(names[0]).replace("glyph_id=42", "glyph_id=65536"),
            *rows[2:],
        ],
        [
            meta,
            census,
            alias_census,
            probe_case(names[0]),
            "TEXPOSE_MATH_TRACE case=case-1 index=0 kind=rule "
            "x=0/1 bottom=0/1 width=0/1 height=1/1",
            *rows[2:],
        ],
    )
    for data in bad_probe_sets:
        try:
            parse_math_probe(data, spec)
        except OracleError:
            pass
        else:
            fail("self-test malformed positioned probe evidence accepted")

    with tempfile.TemporaryDirectory(prefix="texpose-math-self-test-") as raw_temp:
        path = Path(raw_temp) / "result.tsv"
        engine = "This is LuaHBTeX, Version 1.25.7 (MiKTeX 26.5)"
        fingerprint = "|".join(
            [
                "FINGERPRINT",
                engine.encode().hex(),
                "MiKTeX 26.5".encode().hex(),
                "2026-06-01".encode().hex(),
                "2023/08/13 v0.8r".encode().hex(),
                "2025/01/01 v2.9".encode().hex(),
                "2025/01/01 v2.17".encode().hex(),
                "a" * 64,
                "0",
                "test",
                "test-v1",
                spec.census_sha256,
                spec.alias_census_sha256,
            ]
        )

        def lua_case(name: str, *, subfont: int = 0, trace_count: int = 1) -> str:
            sizes = "655360|327680|327680" if name == "case-1" else "655360|458752|327680"
            return f"CASE|{name}|65536|32768|0|{sizes}|1|0|0|0|0|{trace_count}|{subfont}"

        def lua_trace(name: str, *, index: int = 0) -> str:
            return f"TRACE|{name}|{index}|G|42|0|0|655360"

        reference_lines = [fingerprint]
        for name in names:
            reference_lines.extend((lua_case(name), lua_trace(name)))
        path.write_text("\n".join(reference_lines) + "\n", encoding="utf-8")
        parsed, reference = parse_math_results(path, cases, spec)
        if len(parsed) != 21 or not reference.environment_sha256():
            fail("self-test valid reference rejected")

        first_delta = compare_positioned_trace(cases["case-1"], parsed["case-1"])
        if not first_delta.glyphs_match or first_delta.maximum != 0.0:
            fail("self-test identical positioned trace did not compare equal")

        topology_reference = dict(parsed["case-1"])
        topology_reference["trace"] = [
            {"kind": "rule", "x_sp": 0, "bottom_sp": 0, "width_sp": 1, "height_sp": 1}
        ]
        topology_delta = compare_positioned_trace(cases["case-1"], topology_reference)
        if topology_delta.topology_mismatch != "primitive 0 kind glyph/rule":
            fail("self-test positioned topology mismatch was not localized")

        glyph_reference = dict(parsed["case-1"])
        glyph_reference["trace"] = [dict(parsed["case-1"]["trace"][0], glyph_id=43)]
        glyph_delta = compare_positioned_trace(cases["case-1"], glyph_reference)
        if glyph_delta.glyph_mismatches != (
            GlyphMismatch(index=0, texpose_glyph_id=42, reference_glyph_id=43),
        ):
            fail("self-test positioned glyph mismatch was not localized")
        if glyph_delta.glyph_multiset_matches:
            fail("self-test glyph-selection mismatch was misclassified as paint order")
        if glyph_delta.maximum != 0.0:
            fail("self-test glyph mismatch produced a false positioned delta")

        numeric_reference = dict(parsed["case-1"])
        numeric_reference["trace"] = [
            dict(parsed["case-1"]["trace"][0], x_sp=65536)
        ]
        numeric_delta = compare_positioned_trace(cases["case-1"], numeric_reference)
        if numeric_delta.maximum_field != "glyph-x" or abs(numeric_delta.maximum - 0.1) > 1e-12:
            fail("self-test positioned numeric delta was not localized")

        exhaustive_case = {
            "name": "two-glyphs",
            "family": "basic",
            "size": 10,
            "trace": [
                {
                    "kind": "glyph",
                    "glyph_id": 10,
                    "x": Fraction(0),
                    "baseline": Fraction(0),
                    "scale": Fraction(1),
                },
                {
                    "kind": "glyph",
                    "glyph_id": 20,
                    "x": Fraction(1),
                    "baseline": Fraction(0),
                    "scale": Fraction(1),
                },
            ],
        }
        exhaustive_reference = {
            "text_em": 655360.0,
            "trace": [
                {
                    "kind": "glyph",
                    "glyph_id": 11,
                    "x_sp": 0,
                    "baseline_sp": 0,
                    "font_size_sp": 655360,
                },
                {
                    "kind": "glyph",
                    "glyph_id": 21,
                    "x_sp": 720896,
                    "baseline_sp": 0,
                    "font_size_sp": 655360,
                },
            ],
        }
        exhaustive_delta = compare_positioned_trace(
            exhaustive_case, exhaustive_reference
        )
        if exhaustive_delta.glyph_mismatches != (
            GlyphMismatch(index=0, texpose_glyph_id=10, reference_glyph_id=11),
            GlyphMismatch(index=1, texpose_glyph_id=20, reference_glyph_id=21),
        ):
            fail("self-test positioned comparator stopped at the first glyph mismatch")
        if exhaustive_delta.glyph_multiset_matches:
            fail("self-test glyph-selection mismatch was misclassified as paint order")
        if exhaustive_delta.maximum != 0.0:
            fail("self-test compared geometry between different glyph identities")

        reordered_reference = {
            "text_em": 655360.0,
            "trace": [
                {
                    "kind": "glyph",
                    "glyph_id": 20,
                    "x_sp": 655360,
                    "baseline_sp": 0,
                    "font_size_sp": 655360,
                },
                {
                    "kind": "glyph",
                    "glyph_id": 10,
                    "x_sp": 0,
                    "baseline_sp": 0,
                    "font_size_sp": 655360,
                },
            ],
        }
        reordered_delta = compare_positioned_trace(exhaustive_case, reordered_reference)
        if not reordered_delta.glyph_multiset_matches:
            fail("self-test pure paint-order mismatch was classified as glyph selection")
        if reordered_delta.geometry_alignment != "glyph-id":
            fail("self-test pure paint-order mismatch was not realigned by glyph id")
        if reordered_delta.maximum != 0.0:
            fail("self-test identity-realigned geometry did not compare equal")

        reordered_offset_reference = {
            "text_em": 655360.0,
            "trace": [
                dict(reordered_reference["trace"][0], x_sp=720896),
                reordered_reference["trace"][1],
            ],
        }
        reordered_offset_delta = compare_positioned_trace(
            exhaustive_case, reordered_offset_reference
        )
        if (
            reordered_offset_delta.maximum_field != "glyph-x"
            or reordered_offset_delta.maximum_index != 1
            or reordered_offset_delta.maximum_reference_index != 0
            or abs(reordered_offset_delta.maximum - 0.1) > 1e-12
        ):
            fail("self-test identity-realigned positioned delta was not localized")

        reorder_spec = replace(
            spec,
            canonical_trace_glyph_mismatches=(
                ("two-glyphs", ((0, 10, 20), (1, 20, 10))),
            ),
            canonical_trace_deviations=(),
            contractual_profile=True,
        )
        if validate_canonical_positioned_contract(reorder_spec, [reordered_delta]):
            fail("self-test exact reorder unexpectedly reported a trace deviation")
        try:
            validate_canonical_positioned_contract(
                reorder_spec, [reordered_offset_delta]
            )
        except OracleError:
            pass
        else:
            fail("self-test reordered geometry above tolerance was accepted")

        bounded_reorder_spec = replace(
            reorder_spec,
            canonical_trace_deviations=(("two-glyphs", 0.11),),
        )
        if validate_canonical_positioned_contract(
            bounded_reorder_spec, [reordered_offset_delta]
        ) != ["two-glyphs"]:
            fail("self-test bounded identity-realigned reorder was rejected")

        duplicate_case = {
            "name": "duplicate-glyphs",
            "family": "basic",
            "size": 10,
            "trace": [
                dict(exhaustive_case["trace"][0], glyph_id=10),
                dict(exhaustive_case["trace"][1], glyph_id=10),
                dict(exhaustive_case["trace"][1], glyph_id=20),
            ],
        }
        duplicate_reference = {
            "text_em": 655360.0,
            "trace": [
                dict(reordered_reference["trace"][0], glyph_id=20),
                dict(reordered_reference["trace"][1], glyph_id=10),
                dict(reordered_reference["trace"][1], glyph_id=10),
            ],
        }
        duplicate_delta = compare_positioned_trace(
            duplicate_case, duplicate_reference
        )
        if not duplicate_delta.glyph_multiset_matches:
            fail("self-test duplicate pure reorder was classified as glyph selection")
        if duplicate_delta.geometry_comparable:
            fail("self-test ambiguous duplicate-glyph reorder fabricated geometry")
        duplicate_spec = replace(
            spec,
            canonical_trace_glyph_mismatches=(
                ("duplicate-glyphs", ((0, 10, 20), (2, 20, 10))),
            ),
            canonical_trace_deviations=(),
            contractual_profile=True,
        )
        if validate_canonical_positioned_contract(duplicate_spec, [duplicate_delta]):
            fail("self-test documented duplicate-glyph reorder was rejected")
        try:
            validate_canonical_positioned_contract(
                replace(
                    duplicate_spec,
                    canonical_trace_deviations=(("duplicate-glyphs", 0.11),),
                ),
                [duplicate_delta],
            )
        except OracleError:
            pass
        else:
            fail("self-test ambiguous reorder ceiling masked non-comparable geometry")

        mixed_rule_case = {
            "name": "glyph-rule-glyph",
            "family": "basic",
            "size": 10,
            "trace": [
                exhaustive_case["trace"][0],
                {
                    "kind": "rule",
                    "x": Fraction(0),
                    "bottom": Fraction(0),
                    "width": Fraction(1),
                    "height": Fraction(1, 10),
                },
                exhaustive_case["trace"][1],
            ],
        }
        mixed_rule_reference = {
            "text_em": 655360.0,
            "trace": [
                reordered_reference["trace"][0],
                {
                    "kind": "rule",
                    "x_sp": 0,
                    "bottom_sp": 0,
                    "width_sp": 655360,
                    "height_sp": 65536,
                },
                reordered_reference["trace"][1],
            ],
        }
        mixed_rule_delta = compare_positioned_trace(
            mixed_rule_case, mixed_rule_reference
        )
        if not mixed_rule_delta.glyph_multiset_matches:
            fail("self-test mixed-rule pure reorder was classified as glyph selection")
        if mixed_rule_delta.geometry_comparable:
            fail("self-test reordered mixed glyph/rule trace fabricated geometry")
        mixed_rule_spec = replace(
            spec,
            canonical_trace_glyph_mismatches=(
                ("glyph-rule-glyph", ((0, 10, 20), (2, 20, 10))),
            ),
            canonical_trace_deviations=(),
            contractual_profile=True,
        )
        if validate_canonical_positioned_contract(mixed_rule_spec, [mixed_rule_delta]):
            fail("self-test documented mixed glyph/rule reorder was rejected")

        trace_spec = replace(
            spec,
            canonical_trace_glyph_mismatches=(("case-1", ((0, 1, 2),)),),
            canonical_trace_deviations=(("case-2", 0.08),),
            contractual_profile=True,
        )
        trace_contract = [
            TraceCaseDelta(
                name="case-1",
                family="basic",
                size=10,
                primitive_count=1,
                topology_mismatch=None,
                glyph_mismatches=(
                    GlyphMismatch(index=0, texpose_glyph_id=1, reference_glyph_id=2),
                ),
                glyph_multiset_matches=False,
                geometry_alignment=None,
                maximum=0.0,
                maximum_field=None,
                maximum_index=None,
                maximum_reference_index=None,
            ),
            TraceCaseDelta(
                name="case-2",
                family="basic",
                size=10,
                primitive_count=1,
                topology_mismatch=None,
                glyph_mismatches=(),
                glyph_multiset_matches=True,
                geometry_alignment="paint-index",
                maximum=0.07,
                maximum_field="glyph-x",
                maximum_index=0,
                maximum_reference_index=0,
            ),
            TraceCaseDelta(
                name="case-3",
                family="basic",
                size=10,
                primitive_count=1,
                topology_mismatch=None,
                glyph_mismatches=(),
                glyph_multiset_matches=True,
                geometry_alignment="paint-index",
                maximum=0.01,
                maximum_field="glyph-x",
                maximum_index=0,
                maximum_reference_index=0,
            ),
        ]
        if validate_canonical_positioned_contract(trace_spec, trace_contract) != [
            "case-2"
        ]:
            fail("self-test positioned trace ceiling was not reported")

        try:
            validate_canonical_positioned_contract(
                replace(
                    trace_spec,
                    canonical_trace_deviations=(("case-1", 0.08),),
                ),
                trace_contract,
            )
        except OracleError:
            pass
        else:
            fail("self-test glyph-selection ceiling masked non-comparable geometry")

        rejected_trace_contracts = (
            [
                *trace_contract[:2],
                replace(
                    trace_contract[2],
                    glyph_mismatches=(
                        GlyphMismatch(index=0, texpose_glyph_id=9, reference_glyph_id=10),
                    ),
                    glyph_multiset_matches=False,
                    geometry_alignment=None,
                    maximum=0.0,
                    maximum_field=None,
                    maximum_index=None,
                    maximum_reference_index=None,
                ),
            ],
            [
                replace(
                    trace_contract[0],
                    glyph_mismatches=(),
                    glyph_multiset_matches=True,
                    geometry_alignment="paint-index",
                ),
                *trace_contract[1:],
            ],
            [
                replace(
                    trace_contract[0],
                    glyph_mismatches=(
                        GlyphMismatch(index=0, texpose_glyph_id=1, reference_glyph_id=3),
                    ),
                ),
                *trace_contract[1:],
            ],
            [*trace_contract[:2], replace(trace_contract[2], topology_mismatch="count")],
            [trace_contract[0], replace(trace_contract[1], maximum=0.09), trace_contract[2]],
            [trace_contract[0], replace(trace_contract[1], maximum=0.01), trace_contract[2]],
        )
        for candidate in rejected_trace_contracts:
            try:
                validate_canonical_positioned_contract(trace_spec, candidate)
            except OracleError:
                pass
            else:
                fail("self-test canonical positioned contract failed open")

        bad_result_variants: list[tuple[str, list[str]]] = []
        good_lines = path.read_text(encoding="utf-8").splitlines()

        missing_trace = good_lines.copy()
        missing_trace.pop(2)
        bad_result_variants.append(("truncated positioned trace", missing_trace))

        bad_index = good_lines.copy()
        bad_index[2] = bad_index[2].replace("|0|G|", "|1|G|")
        bad_result_variants.append(("nonsequential positioned trace", bad_index))

        bad_kind = good_lines.copy()
        bad_kind[2] = bad_kind[2].replace(
            "|G|42|0|0|655360", "|X|42|0|0|655360"
        )
        bad_result_variants.append(("unknown positioned primitive", bad_kind))

        bad_glyph = good_lines.copy()
        bad_glyph[2] = bad_glyph[2].replace("|G|42|", "|G|65536|")
        bad_result_variants.append(("out-of-range glyph id", bad_glyph))

        bad_rule = good_lines.copy()
        bad_rule[2] = "TRACE|case-1|0|R|0|0|0|65536"
        bad_result_variants.append(("degenerate rule rectangle", bad_rule))

        duplicate_case = good_lines.copy()
        duplicate_case.insert(2, good_lines[1])
        bad_result_variants.append(("duplicate LuaTeX result", duplicate_case))

        missing_case = good_lines[:-2]
        bad_result_variants.append(("missing LuaTeX result", missing_case))

        unexpected_record = [*good_lines, "UNEXPECTED|record"]
        bad_result_variants.append(("unexpected LuaTeX record", unexpected_record))

        nonfinite = good_lines.copy()
        nonfinite_parts = nonfinite[1].split("|")
        nonfinite_parts[2] = "nan"
        nonfinite[1] = "|".join(nonfinite_parts)
        bad_result_variants.append(("non-finite LuaTeX dimension", nonfinite))

        duplicate_fingerprint = [good_lines[0], *good_lines]
        bad_result_variants.append(("duplicate reference fingerprint", duplicate_fingerprint))

        for label, field, value in (
            ("wrong font hash", 7, "b" * 64),
            ("wrong face index", 8, "1"),
            ("wrong profile", 9, "other"),
            ("wrong census", 11, "c" * 64),
            ("wrong alias census", 12, "d" * 64),
        ):
            bad_identity = good_lines.copy()
            identity_parts = bad_identity[0].split("|")
            identity_parts[field] = value
            bad_identity[0] = "|".join(identity_parts)
            bad_result_variants.append((label, bad_identity))

        for label, data in bad_result_variants:
            path.write_text("\n".join(data) + "\n", encoding="utf-8")
            try:
                parse_math_results(path, cases, spec)
            except OracleError:
                pass
            else:
                fail(f"self-test accepted {label}")

        unknown_environment = replace(
            spec, reference_environment_sha256="0" * 64, contractual_profile=True
        )
        try:
            validate_reference_environment(unknown_environment, reference)
        except OracleError:
            pass
        else:
            fail("self-test unknown reference fingerprint accepted")

        collection_spec = RunSpec(
            name="test",
            font_path=Path("unused.ttc"),
            font_sha256="a" * 64,
            face_index=1,
            revision="test-v1",
            measurement_count=21,
            alias_count=21,
            census_sha256=spec.census_sha256,
            alias_census_sha256=spec.alias_census_sha256,
            tolerance=0.05,
            documented_deviations=(),
            canonical_trace_glyph_mismatches=(),
            canonical_trace_deviations=(),
            reference_environment_sha256=None,
            contractual_profile=False,
            collection=True,
        )
        collection_fingerprint = fingerprint.replace("|0|test|", "|1|test|")
        collection_lines = [collection_fingerprint]
        for name in names:
            collection_lines.extend((lua_case(name, subfont=2), lua_trace(name)))
        path.write_text("\n".join(collection_lines) + "\n", encoding="utf-8")
        parse_math_results(path, cases, collection_spec)

        collection_lines[1] = lua_case(names[0], subfont=1)
        path.write_text("\n".join(collection_lines) + "\n", encoding="utf-8")
        try:
            parse_math_results(path, cases, collection_spec)
        except OracleError:
            pass
        else:
            fail("self-test wrong LuaTeX collection face accepted")

    bad_profile = MathProfile(
        name="bad",
        fixture="unused",
        sha256="b" * 64,
        face_index=0,
        required_capabilities=("math-font",),
        capability_exclusions=(("case-1", "unknown-capability"),),
        canonical_measurements=21,
        canonical_aliases=21,
        canonical_census_sha256=CANONICAL_CENSUS_SHA256,
        canonical_alias_census_sha256=CANONICAL_ALIAS_CENSUS_SHA256,
        canonical_tolerance=0.05,
        stress_measurements=89,
        stress_aliases=94,
        stress_census_sha256=STRESS_CENSUS_SHA256,
        stress_alias_census_sha256=STRESS_ALIAS_CENSUS_SHA256,
        stress_tolerance=0.05,
        documented_deviations=(),
        canonical_trace_glyph_mismatches=(),
        canonical_trace_deviations=(),
        reference_environment_sha256=None,
    )
    try:
        validate_profile(bad_profile)
    except OracleError:
        pass
    else:
        fail("self-test unknown capability exclusion accepted")

def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="TeXpose LuaLaTeX differential math oracle"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    math_parser = sub.add_parser(
        "math", help="compare TeXpose layout against LuaLaTeX"
    )
    source = math_parser.add_mutually_exclusive_group()
    source.add_argument("--profile", choices=sorted(PROFILES))
    source.add_argument("--font", type=Path)
    math_parser.add_argument("--face-index", type=int)
    math_parser.add_argument("--stress", action="store_true")
    math_parser.add_argument("--tolerance", type=float)
    math_parser.add_argument("--top-worst", type=int, default=12)
    math_parser.add_argument("--fail-on-delta", action="store_true")

    sub.add_parser("self-test", help="exercise the oracle evidence parsers")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "self-test":
            self_test()
            print("self-test: ok")
        elif args.command == "math":
            gate_math(args=args)
        return 0
    except OracleError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
