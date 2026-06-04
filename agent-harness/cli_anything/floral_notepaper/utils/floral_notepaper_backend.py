from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any


class BackendError(RuntimeError):
    def __init__(self, code: str, message: str, details: dict[str, str] | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or {}


def repository_root() -> Path:
    return Path(__file__).resolve().parents[4]


def backend_executable() -> Path | None:
    configured = os.environ.get("FLORAL_CLI_BACKEND")
    if configured:
        path = Path(configured).expanduser().resolve()
        if path.is_file():
            return path

    suffix = ".exe" if os.name == "nt" else ""
    candidate = repository_root() / "src-tauri" / "target" / "debug" / f"floral_cli_backend{suffix}"
    return candidate if candidate.is_file() else None


def build_backend() -> Path:
    manifest = repository_root() / "src-tauri" / "Cargo.toml"
    command = [
        "cargo",
        "build",
        "--manifest-path",
        str(manifest),
        "--bin",
        "floral_cli_backend",
    ]
    completed = subprocess.run(command, cwd=repository_root(), text=True, capture_output=True)
    if completed.returncode != 0:
        raise BackendError(
            "backendBuildFailed",
            completed.stderr.strip() or "Failed to build the Floral Notepaper backend.",
        )
    executable = backend_executable()
    if executable is None:
        raise BackendError("backendMissing", "The backend build completed but no executable was found.")
    return executable


def call_backend(operation: str, payload: dict[str, Any] | None, data_dir: Path) -> Any:
    executable = backend_executable() or build_backend()
    environment = os.environ.copy()
    environment["FLORAL_NOTEPAPER_DATA_DIR"] = str(data_dir.resolve())
    completed = subprocess.run(
        [str(executable), operation],
        input=json.dumps(payload or {}, ensure_ascii=False),
        cwd=repository_root(),
        env=environment,
        text=True,
        encoding="utf-8",
        capture_output=True,
    )

    output = completed.stdout.strip()
    try:
        envelope = json.loads(output)
    except json.JSONDecodeError as error:
        detail = completed.stderr.strip() or output or "The backend returned no output."
        raise BackendError("backendProtocol", detail) from error

    if not envelope.get("ok"):
        backend_error = envelope.get("error") or {}
        raise BackendError(
            str(backend_error.get("code", "backendError")),
            str(backend_error.get("message", "The Floral Notepaper backend failed.")),
            backend_error.get("details") or {},
        )
    return envelope.get("data")
