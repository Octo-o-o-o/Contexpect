#!/usr/bin/env python3
"""In-memory Path-like tree for read-only acceptance tests.

Tests may snapshot repository files into a dict and mutate the copy.
Validators that accept a Path root can consume MemoryPath without writing
to the worktree or TMPDIR.
"""

from __future__ import annotations

import fnmatch
import io
import sys
from pathlib import Path
from typing import Any, Iterable, Iterator


def _norm(rel: str) -> str:
    rel = str(rel).replace("\\", "/")
    parts: list[str] = []
    for item in rel.split("/"):
        if item in ("", "."):
            continue
        if item == "..":
            if parts and parts[-1] != "..":
                parts.pop()
            else:
                parts.append("..")
        else:
            parts.append(item)
    return "/".join(parts)


class _MemoryStat:
    def __init__(self, size: int) -> None:
        self.st_size = size


class _MemoryFile:
    def __init__(self, store: dict[str, str], rel: str, mode: str, initial: str) -> None:
        self._store = store
        self._rel = rel
        self._mode = mode
        self._buf = io.StringIO(initial if "r" in mode and "w" not in mode else (initial if "a" in mode else ""))
        if "a" in mode:
            self._buf.seek(0, io.SEEK_END)
        self._closed = False

    def write(self, data: str) -> int:
        return self._buf.write(data)

    def read(self, *args: Any) -> str:
        return self._buf.read(*args)

    def flush(self) -> None:
        if "w" in self._mode or "a" in self._mode or "+" in self._mode:
            self._store[self._rel] = self._buf.getvalue()

    def close(self) -> None:
        if not self._closed:
            self.flush()
            self._closed = True
            self._buf.close()

    def __iter__(self) -> Iterator[str]:
        self._buf.seek(0)
        return iter(self._buf)

    def __enter__(self) -> _MemoryFile:
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()


class MemoryPath:
    """Minimal Path-compatible object backed by a shared text-file mapping."""

    def __init__(self, store: dict[str, str], rel: str = "") -> None:
        self._store = store
        self._rel = _norm(rel)

    def __truediv__(self, other: object) -> MemoryPath:
        other_s = str(other).replace("\\", "/").lstrip("/")
        if not self._rel:
            return MemoryPath(self._store, other_s)
        return MemoryPath(self._store, f"{self._rel}/{other_s}")

    def __str__(self) -> str:
        return self._rel or "."

    def __repr__(self) -> str:
        return f"MemoryPath({self._rel!r})"

    def __fspath__(self) -> str:
        return self._rel

    def __eq__(self, other: object) -> bool:
        if isinstance(other, MemoryPath):
            return self._rel == other._rel
        return str(self) == str(other)

    def __hash__(self) -> int:
        return hash(self._rel)

    def __lt__(self, other: object) -> bool:
        return str(self) < str(other)

    @property
    def name(self) -> str:
        return self._rel.rsplit("/", 1)[-1] if self._rel else ""

    @property
    def parent(self) -> MemoryPath:
        if "/" not in self._rel:
            return MemoryPath(self._store, "")
        return MemoryPath(self._store, self._rel.rsplit("/", 1)[0])

    @property
    def parts(self) -> tuple[str, ...]:
        if not self._rel:
            return tuple()
        return tuple(self._rel.split("/"))

    def as_posix(self) -> str:
        return self._rel

    def resolve(self) -> MemoryPath:
        return MemoryPath(self._store, _norm(self._rel))

    def is_file(self) -> bool:
        return self._rel in self._store

    def is_dir(self) -> bool:
        if self._rel in self._store:
            return False
        prefix = f"{self._rel}/" if self._rel else ""
        if not self._rel:
            return True
        return any(key.startswith(prefix) for key in self._store)

    def exists(self) -> bool:
        return self.is_file() or self.is_dir()

    def read_text(self, encoding: str = "utf-8", errors: str | None = None) -> str:
        if self._rel not in self._store:
            raise FileNotFoundError(self._rel)
        return self._store[self._rel]

    def write_text(self, data: str, encoding: str = "utf-8", errors: str | None = None) -> int:
        self._store[self._rel] = data
        return len(data)

    def unlink(self) -> None:
        try:
            del self._store[self._rel]
        except KeyError as exc:
            raise FileNotFoundError(self._rel) from exc

    def mkdir(self, parents: bool = False, exist_ok: bool = False) -> None:
        return None

    def relative_to(self, other: object) -> MemoryPath:
        other_rel = other._rel if isinstance(other, MemoryPath) else _norm(str(other))
        if other_rel in {"", "."}:
            return MemoryPath(self._store, self._rel)
        if self._rel == other_rel:
            return MemoryPath(self._store, "")
        prefix = other_rel + "/"
        if self._rel.startswith(prefix):
            return MemoryPath(self._store, self._rel[len(prefix) :])
        raise ValueError(f"{self._rel} is not relative to {other_rel}")

    def glob(self, pattern: str) -> Iterable[MemoryPath]:
        prefix = f"{self._rel}/" if self._rel else ""
        for key in sorted(self._store):
            if prefix and not key.startswith(prefix):
                continue
            rest = key[len(prefix) :] if prefix else key
            if "/" in rest:
                continue
            if fnmatch.fnmatch(rest, pattern):
                yield MemoryPath(self._store, key)

    def rglob(self, pattern: str) -> Iterable[MemoryPath]:
        prefix = f"{self._rel}/" if self._rel else ""
        for key in sorted(self._store):
            if self._rel and not key.startswith(prefix):
                continue
            name = key.rsplit("/", 1)[-1]
            rest = key[len(prefix) :] if prefix else key
            if pattern == "*":
                yield MemoryPath(self._store, key)
            elif fnmatch.fnmatch(name, pattern) or fnmatch.fnmatch(rest, pattern):
                yield MemoryPath(self._store, key)

    def open(self, mode: str = "r", encoding: str | None = None, newline: str | None = None) -> _MemoryFile:
        initial = self._store.get(self._rel, "")
        if "r" in mode and "w" not in mode and "a" not in mode and self._rel not in self._store:
            raise FileNotFoundError(self._rel)
        return _MemoryFile(self._store, self._rel, mode, initial)

    def stat(self) -> _MemoryStat:
        if self._rel not in self._store:
            raise FileNotFoundError(self._rel)
        return _MemoryStat(len(self._store[self._rel].encode("utf-8")))


def snapshot_paths(repo: Path, rels: list[str]) -> dict[str, str]:
    store: dict[str, str] = {}

    def _read(path: Path, rel: str) -> None:
        # A non-UTF-8 file (e.g. a stray .DS_Store under acceptance/) must
        # not take down the whole snapshot. It is skipped with a note on
        # stderr — never decoded with errors="replace", because callers
        # recompute sha256 digests from the decoded text and a replaced
        # character would silently corrupt them.
        try:
            store[rel] = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            print(f"snapshot_paths: skipping non-UTF-8 file {rel}", file=sys.stderr)

    for rel in rels:
        path = repo / rel
        if path.is_file():
            _read(path, rel)
        elif path.is_dir():
            for child in path.rglob("*"):
                if child.is_file():
                    _read(child, child.relative_to(repo).as_posix())
    return store


def memory_root(store: dict[str, str]) -> MemoryPath:
    return MemoryPath(store, "")
