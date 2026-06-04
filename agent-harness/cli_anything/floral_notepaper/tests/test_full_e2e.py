from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest


COMMAND = "cli-anything-floral-notepaper"


def run_cli(data_dir: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["CLI_ANYTHING_SESSION_DIR"] = str(data_dir.parent / "sessions")
    return subprocess.run(
        [COMMAND, "--data-dir", str(data_dir), "--json", *arguments],
        text=True,
        encoding="utf-8",
        capture_output=True,
        env=environment,
    )


@pytest.mark.e2e
def test_real_backend_note_category_and_history_workflow(tmp_path: Path) -> None:
    data_dir = tmp_path / "data"

    created = run_cli(data_dir, "notes", "create", "--title", "CLI draft", "--content", "first")
    assert created.returncode == 0, created.stderr
    note = json.loads(created.stdout)
    note_id = note["id"]

    category = run_cli(data_dir, "categories", "create", "Work")
    assert category.returncode == 0, category.stderr
    moved = run_cli(data_dir, "notes", "move", note_id, "--category", "Work")
    assert json.loads(moved.stdout)["category"] == "Work"

    updated = run_cli(data_dir, "notes", "update", note_id, "--content", "second")
    assert json.loads(updated.stdout)["content"] == "second"

    undone = run_cli(data_dir, "session", "undo")
    assert undone.returncode == 0, undone.stderr
    restored = run_cli(data_dir, "notes", "get", note_id)
    assert json.loads(restored.stdout)["content"] == "first"

    redone = run_cli(data_dir, "session", "redo")
    assert redone.returncode == 0, redone.stderr
    latest = run_cli(data_dir, "notes", "get", note_id)
    assert json.loads(latest.stdout)["content"] == "second"

    listed = run_cli(data_dir, "notes", "list", "--category", "Work")
    assert [item["id"] for item in json.loads(listed.stdout)] == [note_id]

    config = run_cli(data_dir, "config", "show")
    assert Path(json.loads(config.stdout)["notesDir"]).name == "notes"
