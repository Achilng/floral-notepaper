from __future__ import annotations

import json
from pathlib import Path

from click.testing import CliRunner

from cli_anything.floral_notepaper.core.session import SessionManager
from cli_anything.floral_notepaper.floral_notepaper_cli import cli


def test_session_undo_and_redo_restore_data(tmp_path: Path, monkeypatch) -> None:
    data_dir = tmp_path / "data"
    data_dir.mkdir()
    note = data_dir / "note.md"
    note.write_text("first", encoding="utf-8")
    monkeypatch.setenv("CLI_ANYTHING_SESSION_DIR", str(tmp_path / "sessions"))
    session = SessionManager(data_dir)

    session.record_before_mutation()
    note.write_text("second", encoding="utf-8")

    session.undo()
    assert note.read_text(encoding="utf-8") == "first"

    session.redo()
    assert note.read_text(encoding="utf-8") == "second"


def test_notes_list_filters_and_emits_json(tmp_path: Path, monkeypatch) -> None:
    notes = [
        {"id": "1", "title": "Alpha", "preview": "first", "category": "Work"},
        {"id": "2", "title": "Beta", "preview": "second", "category": "Home"},
    ]
    monkeypatch.setattr(
        "cli_anything.floral_notepaper.floral_notepaper_cli.call_backend",
        lambda operation, payload, data_dir: notes,
    )

    result = CliRunner().invoke(
        cli,
        ["--data-dir", str(tmp_path), "--json", "notes", "list", "--category", "Work"],
    )

    assert result.exit_code == 0
    assert json.loads(result.output) == [notes[0]]


def test_successful_mutation_records_history(tmp_path: Path, monkeypatch) -> None:
    calls: list[str] = []

    def fake_backend(operation, payload, data_dir):
        calls.append(operation)
        return {
            "id": "note-1",
            "title": payload["title"],
            "content": payload["content"],
            "category": payload["category"],
        }

    monkeypatch.setenv("CLI_ANYTHING_SESSION_DIR", str(tmp_path / "sessions"))
    monkeypatch.setattr(
        "cli_anything.floral_notepaper.floral_notepaper_cli.call_backend",
        fake_backend,
    )

    result = CliRunner().invoke(
        cli,
        ["--data-dir", str(tmp_path / "data"), "notes", "create", "--title", "Draft"],
    )

    assert result.exit_code == 0
    assert calls == ["notes.create"]
    status = SessionManager(tmp_path / "data").status()
    assert status.undo_count == 1


def test_failed_mutation_does_not_record_history(tmp_path: Path, monkeypatch) -> None:
    def failing_backend(operation, payload, data_dir):
        raise RuntimeError("write failed")

    monkeypatch.setenv("CLI_ANYTHING_SESSION_DIR", str(tmp_path / "sessions"))
    monkeypatch.setattr(
        "cli_anything.floral_notepaper.floral_notepaper_cli.call_backend",
        failing_backend,
    )

    result = CliRunner().invoke(
        cli,
        ["--data-dir", str(tmp_path / "data"), "notes", "create", "--title", "Draft"],
    )

    assert result.exit_code == 1
    status = SessionManager(tmp_path / "data").status()
    assert status.undo_count == 0


def test_invalid_config_patch_reports_a_cli_error(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CLI_ANYTHING_SESSION_DIR", str(tmp_path / "sessions"))

    result = CliRunner().invoke(
        cli,
        ["--data-dir", str(tmp_path / "data"), "config", "patch", "{bad json}"],
    )

    assert result.exit_code == 1
    assert "Error:" in result.output
    assert "Traceback" not in result.output
