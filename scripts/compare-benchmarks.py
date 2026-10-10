#!/usr/bin/env python3
"""Compare complete, frozen Criterion suites; fail closed on missing or invalid cases."""
import argparse
import json
import math
from pathlib import Path


def read_suite(root, baseline):
    """Read mean estimates and confidence intervals by stable relative benchmark path."""
    result = {}
    for path in Path(root).rglob(f"{baseline}/estimates.json"):
        key = str(path.parent.parent.relative_to(root))
        estimate = json.loads(path.read_text())["mean"]
        mean = estimate["point_estimate"]
        interval = estimate["confidence_interval"]
        values = [mean, interval["lower_bound"], interval["upper_bound"]]
        if not all(math.isfinite(value) and value > 0 for value in values):
            raise ValueError(f"Invalid timing for {key}")
        result[key] = values
    if not result:
        raise ValueError(f"Empty suite: {baseline}")
    return result


def compare(before, after):
    """Equal-weight geometric mean; conservative interval uses every per-case endpoint."""
    if before.keys() != after.keys():
        raise ValueError(f"Suite changed: missing={sorted(before.keys()-after.keys())}, added={sorted(after.keys()-before.keys())}")
    rows = [{"case": key, "before_ns": before[key][0], "after_ns": after[key][0],
             "speedup": before[key][0] / after[key][0],
             "lower": before[key][1] / after[key][2],
             "upper": before[key][2] / after[key][1]} for key in sorted(before)]
    geomean = lambda field: math.exp(sum(math.log(row[field]) for row in rows) / len(rows))
    return {"case_count": len(rows), "geomean_speedup": geomean("speedup"),
            "conservative_lower": geomean("lower"), "conservative_upper": geomean("upper"),
            "cases": rows}


def main():
    """Report all results, returning failure if the requested gate is not reached."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("--root", type=Path, default=Path("target/criterion"))
    parser.add_argument("--minimum", type=float, default=1.5)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = compare(read_suite(args.root, args.before), read_suite(args.root, args.after))
    report["required_speedup"] = args.minimum
    report["passed"] = report["conservative_lower"] >= args.minimum
    rendered = json.dumps(report, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered)
    print(rendered)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
