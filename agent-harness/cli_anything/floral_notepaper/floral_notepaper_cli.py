from __future__ import annotations

import json
import shlex
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable

import click

from .core.session import SessionManager
from .utils.floral_notepaper_backend import BackendError, call_backend


@dataclass
class HarnessContext:
    data_dir: Path
    json_output: bool
    session: SessionManager

    def call(self, operation: str, payload: dict[str, Any] | None = None) -> Any:
        return call_backend(operation, payload, self.data_dir)

    def mutate(self, operation: str, payload: dict[str, Any] | None = None) -> Any:
        snapshot = self.session.begin_mutation()
        try:
            result = self.call(operation, payload)
        except Exception:
            self.session.cancel_mutation(snapshot)
            raise
        self.session.commit_mutation(snapshot)
        return result


def default_data_dir() -> Path:
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "花笺"
    if sys.platform == "win32":
        return Path.home() / "Documents" / "花笺"
    return Path.cwd() / "data"


def emit(context: HarnessContext, value: Any, message: str | None = None) -> None:
    if context.json_output:
        click.echo(json.dumps(value, ensure_ascii=False, indent=2, default=str))
    elif message is not None:
        click.echo(message)
    elif isinstance(value, (dict, list)):
        click.echo(json.dumps(value, ensure_ascii=False, indent=2, default=str))
    elif value is not None:
        click.echo(str(value))


def read_content(content: str | None, content_file: Path | None) -> str:
    if content is not None and content_file is not None:
        raise click.UsageError("Use either --content or --content-file, not both.")
    if content_file is not None:
        return content_file.read_text(encoding="utf-8")
    return content or ""


def handle_errors(function: Callable[..., Any]) -> Callable[..., Any]:
    def wrapped(*args: Any, **kwargs: Any) -> Any:
        try:
            return function(*args, **kwargs)
        except BackendError as error:
            context = click.get_current_context().find_root().obj
            if isinstance(context, HarnessContext) and context.json_output:
                click.echo(
                    json.dumps(
                        {
                            "ok": False,
                            "error": {
                                "code": error.code,
                                "message": error.message,
                                "details": error.details,
                            },
                        },
                        ensure_ascii=False,
                        indent=2,
                    )
                )
                raise click.exceptions.Exit(1) from error
            raise click.ClickException(f"{error.code}: {error.message}") from error
        except (OSError, RuntimeError, ValueError) as error:
            raise click.ClickException(str(error)) from error

    wrapped.__name__ = function.__name__
    wrapped.__doc__ = function.__doc__
    return wrapped


@click.group(invoke_without_command=True)
@click.option("--data-dir", type=click.Path(path_type=Path), default=default_data_dir, show_default=True)
@click.option("--json", "json_output", is_flag=True, help="Emit machine-readable JSON.")
@click.pass_context
def cli(context: click.Context, data_dir: Path, json_output: bool) -> None:
    """Control Floral Notepaper without opening the GUI."""
    harness = HarnessContext(data_dir.resolve(), json_output, SessionManager(data_dir))
    context.obj = harness
    if context.invoked_subcommand is None:
        repl(harness)


@cli.group()
def notes() -> None:
    """Create, inspect, update, move, import, export, and delete notes."""


@notes.command("list")
@click.option("--category", default=None, help="Only show notes in this category.")
@click.option("--query", default=None, help="Search title and preview.")
@click.pass_obj
@handle_errors
def notes_list(context: HarnessContext, category: str | None, query: str | None) -> None:
    result = context.call("notes.list")
    if category is not None:
        result = [note for note in result if note.get("category", "") == category]
    if query:
        lowered = query.lower()
        result = [
            note
            for note in result
            if lowered in note.get("title", "").lower() or lowered in note.get("preview", "").lower()
        ]
    emit(context, result)


@notes.command("get")
@click.argument("note_id")
@click.pass_obj
@handle_errors
def notes_get(context: HarnessContext, note_id: str) -> None:
    emit(context, context.call("notes.get", {"id": note_id}))


@notes.command("create")
@click.option("--title", required=True)
@click.option("--content")
@click.option("--content-file", type=click.Path(exists=True, dir_okay=False, path_type=Path))
@click.option("--category", default="")
@click.pass_obj
@handle_errors
def notes_create(
    context: HarnessContext,
    title: str,
    content: str | None,
    content_file: Path | None,
    category: str,
) -> None:
    result = context.mutate(
        "notes.create",
        {"title": title, "content": read_content(content, content_file), "category": category},
    )
    emit(context, result, f"Created note {result['id']}: {result['title']}")


@notes.command("update")
@click.argument("note_id")
@click.option("--title")
@click.option("--content")
@click.option("--content-file", type=click.Path(exists=True, dir_okay=False, path_type=Path))
@click.option("--category")
@click.pass_obj
@handle_errors
def notes_update(
    context: HarnessContext,
    note_id: str,
    title: str | None,
    content: str | None,
    content_file: Path | None,
    category: str | None,
) -> None:
    current = context.call("notes.get", {"id": note_id})
    updated_content = current["content"] if content is None and content_file is None else read_content(content, content_file)
    result = context.mutate(
        "notes.update",
        {
            "id": note_id,
            "request": {
                "title": current["title"] if title is None else title,
                "content": updated_content,
                "category": current.get("category", "") if category is None else category,
            },
        },
    )
    emit(context, result, f"Updated note {note_id}.")


@notes.command("delete")
@click.argument("note_id")
@click.option("--yes", is_flag=True, help="Skip the confirmation prompt.")
@click.pass_obj
@handle_errors
def notes_delete(context: HarnessContext, note_id: str, yes: bool) -> None:
    if not yes and not click.confirm(f"Delete note {note_id}?"):
        return
    context.mutate("notes.delete", {"id": note_id})
    emit(context, {"deleted": note_id}, f"Deleted note {note_id}.")


@notes.command("move")
@click.argument("note_id")
@click.option("--category", required=True)
@click.pass_obj
@handle_errors
def notes_move(context: HarnessContext, note_id: str, category: str) -> None:
    result = context.mutate("notes.move", {"id": note_id, "category": category})
    emit(context, result, f"Moved note {note_id} to {category or 'uncategorized'}.")


@notes.command("import")
@click.argument("path", type=click.Path(exists=True, dir_okay=False, path_type=Path))
@click.option("--category", default="")
@click.pass_obj
@handle_errors
def notes_import(context: HarnessContext, path: Path, category: str) -> None:
    result = context.mutate("notes.import", {"path": str(path.resolve()), "category": category})
    emit(context, result, f"Imported note {result['id']}.")


@notes.command("export")
@click.argument("note_id")
@click.argument("path", type=click.Path(dir_okay=False, path_type=Path))
@click.pass_obj
@handle_errors
def notes_export(context: HarnessContext, note_id: str, path: Path) -> None:
    context.call("notes.export", {"id": note_id, "path": str(path.resolve())})
    emit(context, {"path": str(path.resolve())}, f"Exported note to {path.resolve()}.")


@cli.group()
def categories() -> None:
    """Manage note categories."""


@categories.command("list")
@click.pass_obj
@handle_errors
def categories_list(context: HarnessContext) -> None:
    emit(context, context.call("categories.list"))


@categories.command("create")
@click.argument("name")
@click.pass_obj
@handle_errors
def categories_create(context: HarnessContext, name: str) -> None:
    context.mutate("categories.create", {"name": name})
    emit(context, {"created": name}, f"Created category {name}.")


@categories.command("rename")
@click.argument("old_name")
@click.argument("new_name")
@click.pass_obj
@handle_errors
def categories_rename(context: HarnessContext, old_name: str, new_name: str) -> None:
    context.mutate("categories.rename", {"oldName": old_name, "newName": new_name})
    emit(context, {"oldName": old_name, "newName": new_name}, f"Renamed {old_name} to {new_name}.")


@categories.command("delete")
@click.argument("name")
@click.option("--yes", is_flag=True, help="Skip the confirmation prompt.")
@click.pass_obj
@handle_errors
def categories_delete(context: HarnessContext, name: str, yes: bool) -> None:
    if not yes and not click.confirm(f"Delete category {name}?"):
        return
    context.mutate("categories.delete", {"name": name})
    emit(context, {"deleted": name}, f"Deleted category {name}.")


@cli.group()
def config() -> None:
    """Inspect or patch application configuration."""


@config.command("show")
@click.pass_obj
@handle_errors
def config_show(context: HarnessContext) -> None:
    emit(context, context.call("config.get"))


@config.command("patch")
@click.argument("patch")
@click.pass_obj
@handle_errors
def config_patch(context: HarnessContext, patch: str) -> None:
    patch_path = Path(patch)
    raw = patch_path.read_text(encoding="utf-8") if patch_path.is_file() else patch
    result = context.mutate("config.patch", {"patch": json.loads(raw)})
    emit(context, result, "Updated application config.")


@cli.group()
def session() -> None:
    """Inspect or navigate harness mutation history."""


@session.command("status")
@click.pass_obj
@handle_errors
def session_status(context: HarnessContext) -> None:
    emit(context, asdict(context.session.status()))


@session.command("undo")
@click.pass_obj
@handle_errors
def session_undo(context: HarnessContext) -> None:
    emit(context, asdict(context.session.undo()), "Undid the most recent mutation.")


@session.command("redo")
@click.pass_obj
@handle_errors
def session_redo(context: HarnessContext) -> None:
    emit(context, asdict(context.session.redo()), "Redid the most recently undone mutation.")


def repl(context: HarnessContext) -> None:
    click.echo("Floral Notepaper CLI. Type 'help' for commands or 'exit' to quit.")
    while True:
        try:
            raw = input("floral> ").strip()
        except (EOFError, KeyboardInterrupt):
            click.echo()
            return
        if not raw:
            continue
        if raw in {"exit", "quit"}:
            return
        if raw == "help":
            cli.main(args=["--help"], standalone_mode=False)
            continue
        arguments = ["--data-dir", str(context.data_dir)]
        if context.json_output:
            arguments.append("--json")
        arguments.extend(shlex.split(raw))
        try:
            cli.main(args=arguments, standalone_mode=False)
        except click.ClickException as error:
            error.show()
        except click.exceptions.Exit:
            continue


def main() -> None:
    cli()
