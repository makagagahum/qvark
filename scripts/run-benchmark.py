#!/usr/bin/env python3
"""Run a reproducible Qorx local-context benchmark.

The benchmark is local-only. It uses Qorx's own CLI, an isolated QORX_HOME, and
the deterministic char/4 token estimator reported by Qorx.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any


DEFAULT_QUERIES = [
    "local context resolution resolver boundary proof page",
    "qorx carriers .qorx .qorxb qorx handle",
    "strict answer refusal unsupported claims",
]

SUPPORTED_QUESTION = "local context resolution resolver boundary proof page"
UNSUPPORTED_QUESTION = "galactic banana escrow treaty"
AGENT_OBJECTIVE = "prove local context resolution resolver boundary"


def run_command(
    args: list[str],
    *,
    env: dict[str, str],
    cwd: Path,
    expect_json: bool,
) -> dict[str, Any]:
    started = time.perf_counter()
    proc = subprocess.run(
        args,
        cwd=str(cwd),
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed_ms = round((time.perf_counter() - started) * 1000, 3)
    record: dict[str, Any] = {
        "command": args,
        "exit_code": proc.returncode,
        "elapsed_ms": elapsed_ms,
        "stderr": proc.stderr.strip(),
    }
    if proc.returncode != 0:
        record["stdout"] = proc.stdout.strip()
        raise RuntimeError(json.dumps(record, indent=2))
    if expect_json:
        record["json"] = json.loads(proc.stdout)
    else:
        record["stdout"] = proc.stdout.strip()
    return record


def safe_clean_qorx_home(repo_root: Path, home: Path) -> None:
    target_root = (repo_root / "target").resolve()
    home = home.resolve()
    if target_root not in home.parents:
        raise RuntimeError(f"refusing to delete benchmark home outside target/: {home}")
    if home.exists():
        shutil.rmtree(home)
    home.mkdir(parents=True, exist_ok=True)


def relative(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return str(path)


def sanitize_for_repo(value: Any, repo_root: Path) -> Any:
    repo = str(repo_root.resolve())
    repo_unc = "\\\\?\\" + repo
    repo_posix = repo_root.resolve().as_posix()

    def clean_text(text: str) -> str:
        cleaned = text.replace(repo_unc, "<repo>")
        cleaned = cleaned.replace(repo, "<repo>")
        cleaned = cleaned.replace(repo_posix, "<repo>")
        return cleaned.replace("\\", "/")

    if isinstance(value, dict):
        return {key: sanitize_for_repo(child, repo_root) for key, child in value.items()}
    if isinstance(value, list):
        return [sanitize_for_repo(child, repo_root) for child in value]
    if isinstance(value, str):
        return clean_text(value)
    return value


def pct(value: float) -> str:
    return f"{value * 100:.1f}%"


def command_for_report(report: dict[str, Any]) -> str:
    parts = [
        "python",
        "scripts/run-benchmark.py",
        "--target",
        report["target_path"],
        "--suite",
        report["suite"],
        "--budget-tokens",
        str(report["budget_tokens"]),
        "--squeeze-budget-tokens",
        str(report["squeeze_budget_tokens"]),
    ]
    for query in report["queries"]:
        parts.extend(["--query", query])
    parts.extend(["--supported-question", report["supported_question"]])
    parts.extend(["--unsupported-question", report["unsupported_question"]])
    parts.extend(["--agent-objective", report["agent_objective"]])
    parts.extend(["--output-json", report["output_json"]])
    parts.extend(["--output-md", report["output_md"]])
    return " ".join(f'"{part}"' if " " in part else part for part in parts)


def write_markdown(report: dict[str, Any], output: Path) -> None:
    summary = report["summary"]
    session = report["session"]["json"]
    bench = report["bench"]["json"]
    pack = report["pack"]["json"]
    squeeze = report["squeeze"]["json"]
    agent = report["agent"]["json"]
    strict_rows = report["strict_answer_tasks"]

    lines = [
        "# Qorx Benchmark Report",
        "",
        f"Generated: `{report['generated_at']}`",
        "",
        f"Suite: `{report['suite']}`",
        "",
        f"Target: `{report['target_path']}`",
        "",
        f"Qorx version: `{report['qorx_version']}`",
        "",
        f"Git commit: `{report['git_commit']}`",
        "",
        "## Summary",
        "",
        "| Metric | Value |",
        "| --- | ---: |",
        f"| Indexed local tokens | {summary['indexed_tokens']} |",
        f"| Session visible tokens | {session['visible_tokens']} |",
        f"| Session reduction | {session['context_reduction_x']:.2f}x |",
        f"| Pack used tokens | {pack['used_tokens']} |",
        f"| Pack reduction | {pack['context_reduction_x']:.2f}x |",
        f"| Squeeze used tokens | {squeeze['used_tokens']} |",
        f"| Squeeze reduction | {squeeze['context_reduction_x']:.2f}x |",
        f"| Bench average reduction | {bench['average_reduction_x']:.2f}x |",
        f"| Strict task pass rate | {pct(summary['strict_task_pass_rate'])} |",
        f"| Expected refusal pass rate | {pct(summary['expected_refusal_pass_rate'])} |",
        f"| Agent provider calls | {agent['provider_calls']} |",
        "",
        "## Strict Tasks",
        "",
        "| Question | Expected | Actual | Pass | Evidence | Used tokens |",
        "| --- | --- | --- | ---: | ---: | ---: |",
    ]
    for row in strict_rows:
        value = row["json"]
        lines.append(
            "| {question} | {expected} | {actual} | {passed} | {evidence} | {used} |".format(
                question=row["question"],
                expected=row["expected_coverage"],
                actual=value["coverage"],
                passed="yes" if row["passed"] else "no",
                evidence=len(value["evidence"]),
                used=value["used_tokens"],
            )
        )

    lines.extend(
        [
            "",
            "## Bench Rows",
            "",
            "| Query | Used tokens | Omitted tokens | Reduction | Quarks |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for row in bench["rows"]:
        lines.append(
            f"| {row['query']} | {row['used_tokens']} | {row['omitted_tokens']} | {row['context_reduction_x']:.2f}x | {row['quarks_used']} |"
        )

    lines.extend(
        [
            "",
            "## Boundary",
            "",
            "This benchmark uses Qorx local accounting only. Token counts are deterministic",
            "`ceil(chars / 4)` estimates unless the runtime reports another estimator. The",
            "report does not claim provider invoice savings, production throughput, or",
            "downstream model answer quality.",
            "",
            "To reproduce:",
            "",
            "```powershell",
            report["reproduce_command"],
            "```",
            "",
        ]
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", default="examples/benchmark-lab")
    parser.add_argument("--suite", default="benchmark-lab")
    parser.add_argument("--budget-tokens", type=int, default=600)
    parser.add_argument("--squeeze-budget-tokens", type=int, default=450)
    parser.add_argument("--output-json", default="docs/benchmarks/2026-05-01-benchmark-lab.json")
    parser.add_argument("--output-md", default="docs/benchmarks/2026-05-01-benchmark-lab.md")
    parser.add_argument("--exe", default="")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument(
        "--query",
        action="append",
        dest="queries",
        help="Benchmark query. May be repeated. Defaults to the benchmark-lab queries.",
    )
    parser.add_argument("--supported-question", default=SUPPORTED_QUESTION)
    parser.add_argument("--unsupported-question", default=UNSUPPORTED_QUESTION)
    parser.add_argument("--agent-objective", default=AGENT_OBJECTIVE)
    args = parser.parse_args()
    queries = args.queries or DEFAULT_QUERIES

    repo_root = Path(__file__).resolve().parents[1]
    target = (repo_root / args.target).resolve()
    if not target.exists():
        raise RuntimeError(f"benchmark target does not exist: {target}")

    exe = Path(args.exe) if args.exe else repo_root / "target" / "release" / "qorx.exe"
    if os.name != "nt" and not args.exe:
        exe = repo_root / "target" / "release" / "qorx"

    if not args.no_build or not exe.exists():
        subprocess.run(["cargo", "build", "--release"], cwd=str(repo_root), check=True)
    if not exe.exists():
        raise RuntimeError(f"qorx executable not found: {exe}")

    qorx_home = repo_root / "target" / f"qorx-benchmark-home-{args.suite}"
    safe_clean_qorx_home(repo_root, qorx_home)

    env = os.environ.copy()
    env["QORX_HOME"] = str(qorx_home)

    git_commit = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=str(repo_root),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    ).stdout.strip()

    version_record = run_command([str(exe), "--version"], env=env, cwd=repo_root, expect_json=False)
    index_record = run_command([str(exe), "index", str(target)], env=env, cwd=repo_root, expect_json=False)
    session_record = run_command([str(exe), "session"], env=env, cwd=repo_root, expect_json=True)
    bench_record = run_command(
        [str(exe), "bench", "--budget-tokens", str(args.budget_tokens), *queries],
        env=env,
        cwd=repo_root,
        expect_json=True,
    )
    pack_record = run_command(
        [str(exe), "pack", args.supported_question, "--budget-tokens", str(args.budget_tokens)],
        env=env,
        cwd=repo_root,
        expect_json=True,
    )
    squeeze_record = run_command(
        [
            str(exe),
            "squeeze",
            args.supported_question,
            "--budget-tokens",
            str(args.squeeze_budget_tokens),
            "--limit",
            "4",
        ],
        env=env,
        cwd=repo_root,
        expect_json=True,
    )
    agent_record = run_command(
        [str(exe), "agent", args.agent_objective, "--budget-tokens", str(args.budget_tokens)],
        env=env,
        cwd=repo_root,
        expect_json=True,
    )

    strict_specs = [
        (args.supported_question, "supported"),
        (args.unsupported_question, "not_found"),
    ]
    strict_rows = []
    for question, expected in strict_specs:
        row = run_command(
            [str(exe), "strict-answer", question, "--limit", "3"],
            env=env,
            cwd=repo_root,
            expect_json=True,
        )
        row["question"] = question
        row["expected_coverage"] = expected
        row["passed"] = row["json"]["coverage"] == expected
        strict_rows.append(row)

    strict_passes = sum(1 for row in strict_rows if row["passed"])
    expected_refusals = [row for row in strict_rows if row["expected_coverage"] == "not_found"]
    refusal_passes = sum(1 for row in expected_refusals if row["passed"])
    indexed_tokens = bench_record["json"]["indexed_tokens"]

    report: dict[str, Any] = {
        "schema": "qorx.evaluation.v1",
        "generated_at": dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat(),
        "suite": args.suite,
        "target_path": relative(target, repo_root),
        "qorx_home": relative(qorx_home, repo_root),
        "git_commit": git_commit,
        "qorx_version": version_record["stdout"],
        "budget_tokens": args.budget_tokens,
        "squeeze_budget_tokens": args.squeeze_budget_tokens,
        "queries": queries,
        "supported_question": args.supported_question,
        "unsupported_question": args.unsupported_question,
        "agent_objective": args.agent_objective,
        "output_json": args.output_json,
        "output_md": args.output_md,
        "summary": {
            "indexed_tokens": indexed_tokens,
            "strict_tasks": len(strict_rows),
            "strict_task_passes": strict_passes,
            "strict_task_pass_rate": strict_passes / max(1, len(strict_rows)),
            "expected_refusals": len(expected_refusals),
            "expected_refusal_passes": refusal_passes,
            "expected_refusal_pass_rate": refusal_passes / max(1, len(expected_refusals)),
            "agent_provider_calls": agent_record["json"]["provider_calls"],
            "boundary": "Local benchmark. Token counts are Qorx deterministic estimates, not provider billing records.",
        },
        "commands": {
            "version": version_record,
            "index": index_record,
        },
        "session": session_record,
        "bench": bench_record,
        "pack": pack_record,
        "squeeze": squeeze_record,
        "agent": agent_record,
        "strict_answer_tasks": strict_rows,
    }
    report["reproduce_command"] = command_for_report(report)

    report = sanitize_for_repo(report, repo_root)

    json_path = repo_root / args.output_json
    md_path = repo_root / args.output_md
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    write_markdown(report, md_path)
    print(f"Wrote {relative(json_path, repo_root)}")
    print(f"Wrote {relative(md_path, repo_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
