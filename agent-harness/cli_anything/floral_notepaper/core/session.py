from __future__ import annotations

import hashlib
import json
import os
import shutil
import uuid
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SessionStatus:
    data_dir: str
    undo_count: int
    redo_count: int


class SessionManager:
    def __init__(self, data_dir: Path):
        self.data_dir = data_dir.resolve()
        digest = hashlib.sha256(str(self.data_dir).encode("utf-8")).hexdigest()[:16]
        root = os.environ.get("CLI_ANYTHING_SESSION_DIR")
        base = Path(root).expanduser() if root else Path.home() / ".cli-anything"
        self.root = base / "floral-notepaper" / digest
        self.state_path = self.root / "state.json"
        self.snapshots_dir = self.root / "snapshots"
        self.root.mkdir(parents=True, exist_ok=True)
        self.snapshots_dir.mkdir(parents=True, exist_ok=True)

    def record_before_mutation(self) -> None:
        snapshot = self.begin_mutation()
        self.commit_mutation(snapshot)

    def begin_mutation(self) -> str:
        return self._create_snapshot()

    def commit_mutation(self, snapshot: str) -> None:
        state = self._load_state()
        state["undo"].append(snapshot)
        for name in state["redo"]:
            self._delete_snapshot(name)
        state["redo"] = []
        self._save_state(state)

    def cancel_mutation(self, snapshot: str) -> None:
        self._delete_snapshot(snapshot)

    def undo(self) -> SessionStatus:
        state = self._load_state()
        if not state["undo"]:
            raise RuntimeError("No operation is available to undo.")
        current = self._create_snapshot()
        target = state["undo"].pop()
        state["redo"].append(current)
        self._restore_snapshot(target)
        self._delete_snapshot(target)
        self._save_state(state)
        return self.status()

    def redo(self) -> SessionStatus:
        state = self._load_state()
        if not state["redo"]:
            raise RuntimeError("No operation is available to redo.")
        current = self._create_snapshot()
        target = state["redo"].pop()
        state["undo"].append(current)
        self._restore_snapshot(target)
        self._delete_snapshot(target)
        self._save_state(state)
        return self.status()

    def status(self) -> SessionStatus:
        state = self._load_state()
        return SessionStatus(
            data_dir=str(self.data_dir),
            undo_count=len(state["undo"]),
            redo_count=len(state["redo"]),
        )

    def _load_state(self) -> dict[str, list[str]]:
        if not self.state_path.exists():
            return {"undo": [], "redo": []}
        return json.loads(self.state_path.read_text(encoding="utf-8"))

    def _save_state(self, state: dict[str, list[str]]) -> None:
        self.state_path.write_text(json.dumps(state, indent=2), encoding="utf-8")

    def _create_snapshot(self) -> str:
        name = uuid.uuid4().hex
        target = self.snapshots_dir / name
        if self.data_dir.exists():
            shutil.copytree(self.data_dir, target)
        else:
            target.mkdir(parents=True)
        return name

    def _restore_snapshot(self, name: str) -> None:
        source = self.snapshots_dir / name
        if self.data_dir.exists():
            shutil.rmtree(self.data_dir)
        shutil.copytree(source, self.data_dir)

    def _delete_snapshot(self, name: str) -> None:
        path = self.snapshots_dir / name
        if path.exists():
            shutil.rmtree(path)
