"""Demonstration of the PyO3 graders against the ArrayList fixture.

Run this after building the extension module (e.g. `maturin develop --features python`).
"""

from __future__ import annotations

from pathlib import Path

from umm import (
    Project,
    ByUnitTestGrader,
    DiffGrader,
    GradeResult,
    QueryGrader,
    UmmError,
    UmmRuntimeError,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_ROOT = REPO_ROOT / "fixtures" / "java" / "arraylist-example"


def show_result(label: str, result: GradeResult) -> None:
    data = result.to_dict()
    print(f"{label}: {data['grade']:.1f}/{data['out_of']:.1f}")
    if data["reason"]:
        print(f"  reason: {data['reason']}")


def build_project() -> Project:
    return Project.from_path(str(FIXTURE_ROOT))


def run_unit_tests(project: Project) -> None:
    try:
        grade = (
            ByUnitTestGrader()
            .project(project)
            .test_files(["DataStructures.ArrayListTest"])
            .out_of(30.0)
            .req_name("JUnit suite")
            .run()
        )
        show_result("ByUnitTestGrader", grade)
    except UmmRuntimeError as err:
        print("ByUnitTestGrader: Java runtime unavailable:", err)


def run_diff(project: Project) -> None:
    # The demo program prints a long deterministic transcript. We keep just the first
    # banner here so the grader can confirm execution without embedding the entire
    # script output.
    expected_prefix = "===== Initial Empty List ====="
    try:
        grade = (
            DiffGrader()
            .project(project)
            .file("Application.Main")
            .req_name("Main banner output")
            .out_of(5.0)
            .ignore_case(True)
            .cases([( "", expected_prefix )])
            .run()
        )
        show_result("DiffGrader", grade)
    except UmmRuntimeError as err:
        print("DiffGrader: Java runtime unavailable:", err)


def run_query(project: Project) -> None:
    grade = (
        QueryGrader()
        .project(project)
        .file("DataStructures.ArrayList")
        .req_name("clear method present")
        .out_of(5.0)
        .reason("clear() should reset the list")
        .method_invocations_with_name("clear")
        .must_match_at_least_once()
        .run()
    )
    show_result("QueryGrader", grade)


if __name__ == "__main__":
    if not FIXTURE_ROOT.exists():
        raise FileNotFoundError(f"Missing fixture directory: {FIXTURE_ROOT}")

    project = build_project()

    try:
        run_unit_tests(project)
        run_diff(project)
        run_query(project)
    except UmmError as err:
        print("Encountered UMM error:", err)
