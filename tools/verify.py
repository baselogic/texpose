#!/usr/bin/env python3
"""LuaLaTeX differential oracle for TeXpose mathematical layout."""

from __future__ import annotations

import argparse
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parent.parent


class OracleError(RuntimeError):
    pass


def fail(message: str) -> None:
    raise OracleError(message)


def require_file(path: Path) -> None:
    if not path.is_file():
        fail(f"required file is missing: {path}")


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


def parse_math_probe(
    lines: Iterable[str], stress: bool
) -> tuple[dict[str, dict[str, object]], int, int]:
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
    params_re = re.compile(
        r"^TEXPOSE_MATH_COMPARE_PARAMS "
        r"script_percent=(\d+) scriptscript_percent=(\d+)$"
    )

    cases: dict[str, dict[str, object]] = {}
    expected: list[str] | None = None
    aliases: set[str] = set()
    script: int | None = None
    scriptscript: int | None = None

    for raw in lines:
        line = raw.rstrip("\r\n")

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

        match = params_re.match(line)
        if match:
            if script is not None:
                fail("duplicate math parameter line")
            script, scriptscript = map(int, match.groups())
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

    if expected is None:
        fail("math probe case census missing")
    if len(expected) != len(set(expected)):
        fail("duplicate name in math probe census")
    if set(expected) != set(cases) or len(expected) != len(cases):
        fail("math probe case census missing/incomplete")

    expected_cases, expected_aliases = (89, 94) if stress else (21, 21)
    if len(cases) != expected_cases or len(aliases) != expected_aliases:
        fail(
            "math profile census changed: "
            f"{len(cases)} measurements, {len(aliases)} aliases"
        )

    if (
        script is None
        or scriptscript is None
        or not (0 < scriptscript <= script <= 100)
    ):
        fail("invalid/missing oracle font script parameters")

    return cases, script, scriptscript


def build_math_tex(
    cases: dict[str, dict[str, object]],
    result: str,
    script: int,
    scriptscript: int,
    font_name: str,
) -> str:
    out = [
        r"\documentclass{article}",
        r"\usepackage{unicode-math}",
        rf"\setmathfont{{{font_name}}}[Path=./]",
    ]

    for size in sorted({int(case["size"]) for case in cases.values()}):
        out.append(
            rf"\DeclareMathSizes{{{size}}}{{{size}}}"
            rf"{{{size * script / 100:.6f}}}"
            rf"{{{size * scriptscript / 100:.6f}}}"
        )

    out += [
        r"\pagestyle{empty}",
        r"\mathsurround=0pt",
        rf"\directlua{{texpose_math_result_file='{result}'; "
        r"dofile('math_compare.lua')}",
        r"\begin{document}",
    ]

    for case in cases.values():
        name = str(case["name"])
        source = str(case["source"])
        if re.search(r"[\r\n%]", source) or not re.fullmatch(
            r"[A-Za-z0-9_-]+", name
        ):
            fail(f"unsafe math fixture: {name}")

        style = (
            r"\displaystyle"
            if case["style"] == "display"
            else r"\textstyle"
        )
        size = int(case["size"])
        out += [
            r"\begingroup",
            rf"\fontsize{{{size}pt}}{{{size}pt}}\selectfont",
            rf"\setbox0=\hbox{{$" + style + " " + source + "$}",
            r"\setbox2=\hbox{$\textstyle x$}",
            r"\dimen0=1em",
            rf"\directlua{{texpose_measure_math_case('{name}', 0, 2, "
            r"tex.dimen[0])}",
            r"\endgroup",
        ]

    out.append(r"\end{document}")
    return "\n".join(out) + "\n"


def parse_math_results(
    path: Path, cases: dict[str, dict[str, object]]
) -> dict[str, dict[str, float | int]]:
    require_file(path)
    result: dict[str, dict[str, float | int]] = {}

    for line in path.read_text(encoding="utf-8").splitlines():
        parts = line.split("|")
        if (
            len(parts) != 12
            or parts[0] != "CASE"
            or parts[1] not in cases
        ):
            fail(f"unexpected/malformed LuaTeX math result: {line}")

        name = parts[1]
        if name in result:
            fail(f"duplicate LuaTeX math case: {name}")

        text_em = parse_float(parts[5])
        math_em = parse_float(parts[11])
        if text_em <= 0 or math_em <= 0:
            fail(f"non-positive text/math em: {name}")

        size = int(cases[name]["size"])
        if abs(math_em - size * 65536) > 2:
            fail(f"LuaTeX selected wrong math size for {name}")

        result[name] = {
            "width": parse_float(parts[2]) / math_em,
            "ascent": parse_float(parts[3]) / math_em,
            "descent": parse_float(parts[4]) / math_em,
            "text_em": text_em,
            "glyphs": parse_int(parts[6], "glyph count"),
            "rules": parse_int(parts[7], "rule count"),
            "hlists": parse_int(parts[8], "hlist count"),
            "vlists": parse_int(parts[9], "vlist count"),
            "max_depth": parse_int(parts[10], "max depth"),
            "math_em": math_em,
        }

    if len(result) != len(cases):
        fail(
            f"expected {len(cases)} LuaTeX cases, got {len(result)}"
        )
    return result


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
        fail(
            f"command failed ({process.returncode}): "
            f"{' '.join(command)}\n{tail}"
        )
    return lines


def percentile(values: list[float], percent: int) -> float:
    if not values:
        return 0.0
    index = math.ceil(percent / 100 * len(values)) - 1
    return values[max(0, min(len(values) - 1, index))]


def gate_math(
    *,
    stress: bool,
    tolerance: float,
    top_worst: int,
    fail_on_delta: bool,
) -> None:
    if not math.isfinite(tolerance) or tolerance < 0:
        fail("tolerance must be finite/nonnegative")
    if not 1 <= top_worst <= 100:
        fail("--top-worst must be 1..100")

    require_file(ROOT / "Cargo.toml")
    require_file(ROOT / "tools" / "math_compare.lua")

    cargo = shutil.which("cargo")
    lualatex = shutil.which("lualatex")
    if cargo is None:
        fail("cargo is not available on PATH")
    if lualatex is None:
        fail("lualatex is not available on PATH")

    with tempfile.TemporaryDirectory(prefix="texpose-math-") as raw_work:
        work = Path(raw_work)
        shutil.copy2(
            ROOT / "tools" / "math_compare.lua",
            work / "math_compare.lua",
        )
        fixture = (
            ROOT
            / "tests"
            / "fixtures"
            / "fonts"
            / "stix-two-math"
            / "STIXTwoMath-Regular.otf"
        )
        require_file(fixture)
        font = work / fixture.name
        shutil.copy2(fixture, font)
        target = work / "target"

        env = {
            "CARGO_TARGET_DIR": str(target),
        }
        if stress:
            env["TEXPOSE_MATH_COMPARE_STRESS"] = "1"

        print("Math oracle: TeXpose probe", file=sys.stderr)
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

        cases, script, scriptscript = parse_math_probe(lines, stress)
        require_file(font)

        tex = work / "math-compare.tex"
        result = work / "math-compare.tsv"
        tex.write_text(
            build_math_tex(
                cases,
                result.name,
                script,
                scriptscript,
                font.name,
            ),
            encoding="utf-8",
        )

        print("Math oracle: LuaLaTeX reference", file=sys.stderr)
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
        reference = parse_math_results(result, cases)

        deltas: list[tuple[float, str, str, int]] = []
        divergent: list[str] = []
        structural: list[str] = []

        for case in cases.values():
            name = str(case["name"])
            expected = reference[name]
            delta = max(
                abs(float(case["width"]) - float(expected["width"])),
                abs(float(case["ascent"]) - float(expected["ascent"])),
                abs(float(case["descent"]) - float(expected["descent"])),
            )
            deltas.append(
                (
                    delta,
                    name,
                    str(case["family"]),
                    int(case["size"]),
                )
            )
            if delta > tolerance:
                divergent.append(name)
            if (
                int(case["glyphs"]) != int(expected["glyphs"])
                or int(case["rules"]) != int(expected["rules"])
            ):
                structural.append(name)

        values = sorted(delta for delta, *_ in deltas)
        maximum = max(values) if values else 0.0
        print(
            f"{len(cases) - len(divergent)}/{len(cases)} "
            f"≤{tolerance:.3f}em · "
            f"p95 {percentile(values, 95):.6f} · "
            f"max {maximum:.6f}"
        )

        if structural:
            print(
                "structure mismatches: " + ", ".join(structural),
                file=sys.stderr,
            )

        if stress or (fail_on_delta and divergent):
            for delta, name, family, size in sorted(
                deltas, reverse=True
            )[:top_worst]:
                if stress or delta > tolerance:
                    print(
                        f"  {name} · {family} · {size}pt · "
                        f"{delta:.6f}em",
                        file=sys.stderr,
                    )

        if fail_on_delta and divergent:
            fail(
                f"math comparison exceeded {tolerance}em: "
                + ", ".join(divergent)
            )


def self_test() -> None:
    names = [f"case-{index}" for index in range(1, 22)]
    head = (
        "TEXPOSE_MATH_COMPARE_PARAMS "
        "script_percent=70 scriptscript_percent=50"
    )
    census = "TEXPOSE_MATH_COMPARE_CASES names=" + ",".join(names)
    rows = [
        "TEXPOSE_MATH_COMPARE "
        f"case={name} family=basic aliases={name} style=text "
        "size_pt=10 source_utf8_hex=78 width_em=1.0 "
        "ascent_em=0.5 descent_em=0.0 glyphs=1 rules=0 ops=1"
        for name in names
    ]

    cases, _, _ = parse_math_probe(
        [head, census, *rows], stress=False
    )
    if len(cases) != 21:
        fail("self-test valid census rejected")

    bad_sets = (
        [head, census, *rows[:-1]],
        [head, census, *rows, rows[0]],
        [head, *rows],
        [head, census, *rows, "TEXPOSE_MATH_COMPARE broken"],
    )
    for data in bad_sets:
        try:
            parse_math_probe(data, stress=False)
        except OracleError:
            pass
        else:
            fail("self-test malformed evidence accepted")

    with tempfile.TemporaryDirectory(
        prefix="texpose-math-self-test-"
    ) as raw_temp:
        path = Path(raw_temp) / "result.tsv"
        path.write_text(
            "\n".join(
                f"CASE|{name}|65536|32768|0|655360|"
                "1|0|0|0|0|655360"
                for name in names
            )
            + "\n",
            encoding="utf-8",
        )
        if len(parse_math_results(path, cases)) != 21:
            fail("self-test valid reference rejected")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="TeXpose LuaLaTeX differential math oracle"
    )
    sub = parser.add_subparsers(dest="command", required=True)

    math_parser = sub.add_parser(
        "math", help="compare TeXpose layout against LuaLaTeX"
    )
    math_parser.add_argument("--stress", action="store_true")
    math_parser.add_argument("--tolerance", type=float, default=0.05)
    math_parser.add_argument("--top-worst", type=int, default=12)
    math_parser.add_argument("--fail-on-delta", action="store_true")

    sub.add_parser(
        "self-test", help="exercise the oracle evidence parsers"
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "self-test":
            self_test()
            print("self-test: ok")
        elif args.command == "math":
            gate_math(
                stress=args.stress,
                tolerance=args.tolerance,
                top_worst=args.top_worst,
                fail_on_delta=args.fail_on_delta,
            )
        return 0
    except OracleError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        return 130


if __name__ == "__main__":
    raise SystemExit(main())
