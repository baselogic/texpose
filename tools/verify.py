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
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parent.parent
PROFILE_REVISION = "oracle-v1"
CANONICAL_CENSUS_SHA256 = "d1d1e356e5a4f426ebf603ed6be5cb134dca3bebe9be7f6b49938ed5ada4ddf1"
STRESS_CENSUS_SHA256 = "383828f9f734c65e67ce7b9a4f12f501fc2951825ed8ffd1ff6fca945b5e29b4"
CANONICAL_ALIAS_CENSUS_SHA256 = CANONICAL_CENSUS_SHA256
STRESS_ALIAS_CENSUS_SHA256 = "1eae50562863db2a26d7c28c96c78a82c3a47649348cb277fc7f17b7578f3b71"
KNOWN_CAPABILITIES = frozenset({"math-font", "canonical-corpus", "stress-corpus"})
REFERENCE_ENVIRONMENT_SHA256 = "b621bc874d9749432eca9f8a66a8bc8ffd72afda0ef8de8624f6a2c2171acbdf"


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
    reference_environment_sha256: str | None
    contractual_profile: bool
    collection: bool


@dataclass(frozen=True)
class ProbeMeta:
    profile: str
    revision: str
    font_sha256: str
    face_index: int
    control_glyph_id: int


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
        documented_deviations=(),
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
                "radical-index",
                Deviation(
                    geometry_ceiling=0.0811,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.081000em width delta; "
                        "Phase G owns radical geometry. The ceiling freezes the "
                        "observed divergence rather than accepting it as correct."
                    ),
                ),
            ),
            (
                "radical-index-compound",
                Deviation(
                    geometry_ceiling=0.0811,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.081001em width delta; "
                        "Phase G owns radical geometry. The ceiling freezes the "
                        "observed divergence rather than accepting it as correct."
                    ),
                ),
            ),
            (
                "radical-plain",
                Deviation(
                    geometry_ceiling=0.0811,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.081001em width delta; "
                        "Phase G owns radical geometry. The ceiling freezes the "
                        "observed divergence rather than accepting it as correct."
                    ),
                ),
            ),
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
        documented_deviations=(
            (
                "accent-widehat-script",
                Deviation(
                    geometry_ceiling=0.1271,
                    allow_structure=False,
                    note=(
                        "MiKTeX 26.5 reference measured 0.127001em descent delta; "
                        "Phase G owns script-style accent geometry. The ceiling "
                        "freezes the observed divergence rather than accepting it "
                        "as correct."
                    ),
                ),
            ),
        ),
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
        r"ops=(?P<ops>[0-9]+)$"
    )
    meta_re = re.compile(
        r"^TEXPOSE_MATH_COMPARE_META "
        r"profile=(?P<profile>[A-Za-z0-9_-]+) "
        r"revision=(?P<revision>[A-Za-z0-9_.-]+) "
        r"font_sha256=(?P<hash>[0-9a-f]{64}) "
        r"face_index=(?P<face>[0-9]+) "
        r"control_glyph_id=(?P<glyph>[0-9]+)$"
    )

    cases: dict[str, dict[str, object]] = {}
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
                control_glyph_id=int(group["glyph"]),
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
        if not match:
            if line.startswith("TEXPOSE_MATH_COMPARE"):
                fail(f"malformed math probe record: {line}")
            continue

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
        }

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
) -> tuple[dict[str, dict[str, float | int]], ReferenceFingerprint]:
    require_file(path)
    result: dict[str, dict[str, float | int]] = {}
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

        if len(parts) != 15 or parts[0] != "CASE" or parts[1] not in cases:
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

        control_glyph_id = parse_int(parts[13], "control glyph id")
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
            "control_glyph_id": control_glyph_id,
            "control_subfont": control_subfont,
        }

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

        environment_sha256 = fingerprint.environment_sha256()
        print(
            "reference fingerprint: "
            f"engine={fingerprint.engine!r} | distribution={fingerprint.distribution!r} | "
            f"latex={fingerprint.latex!r} | unicode-math={fingerprint.unicode_math!r} | "
            f"fontspec={fingerprint.fontspec!r} | amsmath={fingerprint.amsmath!r} | "
            f"sha256={environment_sha256}",
            file=sys.stderr,
        )
        if spec.reference_environment_sha256 is not None:
            if environment_sha256 != spec.reference_environment_sha256:
                fail(
                    f"unknown reference fingerprint for profile {spec.name}: "
                    f"expected {spec.reference_environment_sha256}, got {environment_sha256}"
                )
        elif spec.contractual_profile:
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
    rows = [
        "TEXPOSE_MATH_COMPARE "
        f"case={name} family=basic aliases={name} style=text "
        "size_pt=10 source_utf8_hex=78 width_em=1.0 "
        "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1"
        for name in names
    ]
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
        reference_environment_sha256=None,
        contractual_profile=False,
        collection=False,
    )
    meta = (
        "TEXPOSE_MATH_COMPARE_META profile=test revision=test-v1 "
        f"font_sha256={'a' * 64} face_index=0 control_glyph_id=42"
    )

    cases, parsed_meta = parse_math_probe([meta, census, alias_census, *rows], spec)
    if len(cases) != 21 or parsed_meta.control_glyph_id != 42:
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
        "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1",
        "TEXPOSE_MATH_COMPARE case=single family=basic aliases=single "
        "style=text size_pt=10 source_utf8_hex=79 width_em=1.0 "
        "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1",
    ]
    parse_math_probe(grouped_probe, grouped_spec)

    bad_sets = (
        [census, alias_census, *rows],
        [meta, census, alias_census, *rows[:-1]],
        [meta, census, alias_census, *rows, rows[0]],
        [meta, *rows],
        [meta, census, alias_census, *rows, "TEXPOSE_MATH_COMPARE broken"],
    )
    for data in bad_sets:
        try:
            parse_math_probe(data, spec)
        except OracleError:
            pass
        else:
            fail("self-test malformed probe evidence accepted")

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
        path.write_text(
            fingerprint
            + "\n"
            + "\n".join(
                (
                    f"CASE|{name}|65536|32768|0|655360|327680|327680|"
                    if name == "case-1"
                    else f"CASE|{name}|65536|32768|0|655360|458752|327680|"
                )
                + "1|0|0|0|0|42|0"
                for name in names
            )
            + "\n",
            encoding="utf-8",
        )
        parsed, reference = parse_math_results(path, cases, spec)
        if len(parsed) != 21 or not reference.environment_sha256():
            fail("self-test valid reference rejected")

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
            reference_environment_sha256=None,
            contractual_profile=False,
            collection=True,
        )
        collection_fingerprint = fingerprint.replace("|0|test|", "|1|test|")
        collection_rows = path.read_text(encoding="utf-8").splitlines()[1:]
        collection_rows = [row.rsplit("|", 1)[0] + "|2" for row in collection_rows]
        path.write_text(
            collection_fingerprint + "\n" + "\n".join(collection_rows) + "\n",
            encoding="utf-8",
        )
        parse_math_results(path, cases, collection_spec)

        collection_rows[0] = collection_rows[0].rsplit("|", 1)[0] + "|1"
        path.write_text(
            collection_fingerprint + "\n" + "\n".join(collection_rows) + "\n",
            encoding="utf-8",
        )
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
