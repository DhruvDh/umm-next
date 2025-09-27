from __future__ import annotations

import json
from pathlib import Path

import pytest

import umm
from umm import ByUnitTestGrader, DiffGrader, Project, QueryGrader, UmmError

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE_ROOT = REPO_ROOT / "fixtures" / "java" / "arraylist-example"


@pytest.fixture(scope="module")
def project() -> Project:
    return Project.from_path(str(FIXTURE_ROOT))


def test_module_exports():
    names = {name for name in dir(umm) if not name.startswith("_")}
    assert {"Project", "DiffGrader", "ByUnitTestGrader", "QueryGrader", "GradeResult"} <= names


def test_project_file_inventory(project: Project):
    file_names = set(project.file_names())
    assert "Application.Main" in file_names
    assert "DataStructures.ArrayList" in file_names
    assert project.describe().startswith("<project")


def test_diff_grader_rejects_non_pair_cases(project: Project):
    grader = (
        DiffGrader()
        .project(project)
        .file("Application.Main")
        .req_name("banner")
        .out_of(5.0)
    )
    with pytest.raises(UmmError) as excinfo:
        grader.cases(["not-a-pair"])
    assert "Expected an iterable" in str(excinfo.value)


def test_query_grader_missing_file_exposes_prompt(project: Project):
    result = (
        QueryGrader()
        .project(project)
        .file("Missing")
        .req_name("missing file")
        .out_of(1.0)
        .reason("missing file should trigger prompt context")
        .method_invocations_with_name("clear")
        .must_match_at_least_once()
        .run()
    )
    assert result.grade == 0.0
    prompt_payload = result.prompt_json
    assert prompt_payload
    messages = json.loads(prompt_payload)
    assert any("could not be found" in msg.get("content", "") for msg in messages)


def test_by_unit_test_grader_chain_returns_self(project: Project):
    grader = (
        ByUnitTestGrader()
        .project(project)
        .test_files(["DataStructures.ArrayListTest"])
        .expected_tests(["testClear"])
        .out_of(10.0)
        .req_name("unit tests")
    )
    assert isinstance(grader, ByUnitTestGrader)
