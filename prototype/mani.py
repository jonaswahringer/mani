#!/usr/bin/env python3
"""PROTOTYPE — throw away after choosing Mani's terminal information hierarchy.

Three TUI variants, switchable with Left/Right, for the same command help.
"""

import argparse
import curses
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import List, Sequence, Tuple


PROTOTYPE_DIR = Path(__file__).resolve().parent
VARIANTS = ("A · Minimal pager", "B · Outline browser", "C · Dual source")
CUSTOM = "CUSTOM"
OFFICIAL = "OFFICIAL"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Throwaway Mani TUI prototype")
    parser.add_argument("command", nargs="+", help="command to explain, e.g. git rebase")
    parser.add_argument("--short", action="store_true", help="print concise help to stdout")
    return parser.parse_args()


def knowledge_path(command: Sequence[str]) -> Path:
    return PROTOTYPE_DIR / "kb" / ("-".join(command) + ".md")


def custom_document(command: Sequence[str]) -> Tuple[str, str]:
    path = knowledge_path(command)
    if path.exists():
        return path.read_text(), str(path)

    title = " ".join(command)
    return (
        "# No custom guide\n\n"
        "Mani does not have a custom guide for `%s` yet.\n\n"
        "## Next step\n\nCreate `%s` to add one.\n" % (title, path),
        str(path),
    )


def clean_terminal_text(value: str) -> str:
    ansi = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
    value = ansi.sub("", value)
    while re.search(r".\x08", value):
        value = re.sub(r".\x08", "", value)
    return value.replace("\t", "    ").strip()


def official_document(command: Sequence[str]) -> Tuple[str, str]:
    topic = "-".join(command)
    env = dict(os.environ, MANPAGER="cat", PAGER="cat", MANWIDTH="88")

    try:
        man_result = subprocess.run(
            ["man", topic],
            capture_output=True,
            text=True,
            env=env,
            timeout=4,
        )
        if man_result.returncode == 0 and man_result.stdout.strip():
            return clean_terminal_text(man_result.stdout), "man %s" % topic
    except (OSError, subprocess.TimeoutExpired):
        pass

    try:
        help_result = subprocess.run(
            list(command) + ["--help"],
            capture_output=True,
            text=True,
            timeout=4,
        )
        output = help_result.stdout or help_result.stderr
        if output.strip():
            return clean_terminal_text(output), "%s --help" % " ".join(command)
    except (OSError, subprocess.TimeoutExpired):
        pass

    return "No man page or --help output was found.", "unavailable"


def short_output(command: Sequence[str]) -> int:
    custom, source = custom_document(command)
    if not custom.startswith("# No custom guide"):
        print(custom.rstrip())
        return 0

    try:
        result = subprocess.run(list(command) + ["--help"], text=True)
        return result.returncode
    except OSError as error:
        print("mani: %s" % error, file=sys.stderr)
        return 1


def markdown_lines(document: str) -> List[Tuple[str, str]]:
    rendered = []
    in_code = False
    for raw_line in document.splitlines():
        line = raw_line.rstrip()
        if line.startswith("```"):
            in_code = not in_code
            continue
        if in_code:
            rendered.append(("  " + line, "code"))
        elif line.startswith("# "):
            rendered.append((line[2:].upper(), "title"))
        elif line.startswith("## "):
            rendered.append((line[3:].upper(), "heading"))
        elif line.startswith("- "):
            rendered.append(("  • " + line[2:], "body"))
        elif re.match(r"^\d+\. ", line):
            rendered.append(("  " + line, "body"))
        else:
            rendered.append((line, "body"))
    return rendered


def plain_lines(document: str) -> List[Tuple[str, str]]:
    return [(line, "body") for line in document.splitlines()]


def clipped(value: str, width: int) -> str:
    if width <= 0:
        return ""
    return value[:width]


class Prototype:
    def __init__(self, command: Sequence[str]) -> None:
        self.command = list(command)
        custom, custom_source = custom_document(command)
        official, official_source = official_document(command)
        self.documents = {CUSTOM: custom, OFFICIAL: official}
        self.sources = {CUSTOM: custom_source, OFFICIAL: official_source}
        self.mode = CUSTOM if not custom.startswith("# No custom guide") else OFFICIAL
        self.variant = 0
        self.scroll = {CUSTOM: 0, OFFICIAL: 0}

    def run(self, screen: "curses._CursesWindow") -> None:
        try:
            curses.curs_set(0)
        except curses.error:
            pass
        screen.keypad(True)
        if curses.has_colors():
            curses.start_color()
            curses.use_default_colors()
            curses.init_pair(1, curses.COLOR_GREEN, -1)
            curses.init_pair(2, curses.COLOR_CYAN, -1)
            curses.init_pair(3, curses.COLOR_BLACK, curses.COLOR_GREEN)
            curses.init_pair(4, curses.COLOR_YELLOW, -1)

        while True:
            self.draw(screen)
            key = screen.getch()
            if key in (ord("q"), 27):
                return
            if key == 9:
                self.mode = OFFICIAL if self.mode == CUSTOM else CUSTOM
            elif key == curses.KEY_LEFT:
                self.variant = (self.variant - 1) % len(VARIANTS)
            elif key == curses.KEY_RIGHT:
                self.variant = (self.variant + 1) % len(VARIANTS)
            elif key in (curses.KEY_DOWN, ord("j")):
                self.scroll[self.mode] += 1
            elif key in (curses.KEY_UP, ord("k")):
                self.scroll[self.mode] = max(0, self.scroll[self.mode] - 1)
            elif key == curses.KEY_NPAGE:
                self.scroll[self.mode] += max(1, screen.getmaxyx()[0] - 6)
            elif key == curses.KEY_PPAGE:
                self.scroll[self.mode] = max(0, self.scroll[self.mode] - max(1, screen.getmaxyx()[0] - 6))

    def add(self, screen: "curses._CursesWindow", y: int, x: int, value: str, width: int, style: int = 0) -> None:
        height, total_width = screen.getmaxyx()
        if y < 0 or y >= height or x < 0 or x >= total_width or width <= 0:
            return
        try:
            screen.addnstr(y, x, value, min(width, total_width - x - 1), style)
        except curses.error:
            pass

    def style(self, kind: str) -> int:
        if kind == "title":
            return curses.A_BOLD | curses.color_pair(1)
        if kind == "heading":
            return curses.A_BOLD | curses.color_pair(2)
        if kind == "code":
            return curses.color_pair(4)
        return 0

    def tabs(self) -> str:
        custom = "[ CUSTOM ]" if self.mode == CUSTOM else "  Custom  "
        official = "[ OFFICIAL ]" if self.mode == OFFICIAL else "  Official  "
        return "%s  %s" % (custom, official)

    def draw(self, screen: "curses._CursesWindow") -> None:
        screen.erase()
        height, width = screen.getmaxyx()
        if height < 10 or width < 48:
            self.add(screen, 0, 0, "Make the terminal at least 48×10", width, curses.A_BOLD)
            screen.refresh()
            return

        if self.variant == 0:
            self.draw_minimal(screen, height, width)
        elif self.variant == 1:
            self.draw_outline(screen, height, width)
        else:
            self.draw_dual(screen, height, width)
        screen.refresh()

    def header(self, screen: "curses._CursesWindow", width: int, variant: str) -> None:
        command = " ".join(self.command)
        self.add(screen, 0, 0, " mani  %s" % command, width, curses.A_BOLD | curses.color_pair(3))
        self.add(screen, 1, 1, self.tabs(), width - 2, curses.A_BOLD)
        right = "PROTOTYPE · %s" % variant
        self.add(screen, 1, max(1, width - len(right) - 2), right, len(right), curses.A_DIM)
        self.add(screen, 2, 0, "─" * width, width, curses.A_DIM)

    def footer(self, screen: "curses._CursesWindow", height: int, width: int) -> None:
        label = " ←→ layout   Tab source   ↑↓ scroll   q quit "
        self.add(screen, height - 1, 0, label.ljust(width), width, curses.A_REVERSE)

    def draw_minimal(self, screen: "curses._CursesWindow", height: int, width: int) -> None:
        self.header(screen, width, "A · Minimal pager")
        source = "%s · %s" % (self.mode, self.sources[self.mode])
        self.add(screen, 3, 2, source, width - 4, curses.A_DIM)
        lines = markdown_lines(self.documents[self.mode]) if self.mode == CUSTOM else plain_lines(self.documents[self.mode])
        available = height - 6
        offset = min(self.scroll[self.mode], max(0, len(lines) - available))
        self.scroll[self.mode] = offset
        for row, (text, kind) in enumerate(lines[offset : offset + available], start=5):
            self.add(screen, row, 3, text, width - 6, self.style(kind))
        self.footer(screen, height, width)

    def draw_outline(self, screen: "curses._CursesWindow", height: int, width: int) -> None:
        self.header(screen, width, "B · Outline browser")
        sidebar = min(24, max(18, width // 4))
        lines = markdown_lines(self.documents[self.mode]) if self.mode == CUSTOM else plain_lines(self.documents[self.mode])
        headings = [text.title() for text, kind in lines if kind in ("title", "heading")]
        self.add(screen, 3, 1, "ON THIS PAGE", sidebar - 2, curses.A_BOLD | curses.color_pair(1))
        for row, heading in enumerate(headings[: height - 6], start=5):
            self.add(screen, row, 2, heading, sidebar - 3, curses.A_BOLD if row == 5 else curses.A_DIM)
        for row in range(3, height - 1):
            self.add(screen, row, sidebar, "│", 1, curses.A_DIM)

        self.add(screen, 3, sidebar + 2, "%s · %s" % (self.mode, self.sources[self.mode]), width - sidebar - 4, curses.A_DIM)
        available = height - 6
        offset = min(self.scroll[self.mode], max(0, len(lines) - available))
        self.scroll[self.mode] = offset
        for row, (text, kind) in enumerate(lines[offset : offset + available], start=5):
            self.add(screen, row, sidebar + 2, text, width - sidebar - 4, self.style(kind))
        self.footer(screen, height, width)

    def draw_dual(self, screen: "curses._CursesWindow", height: int, width: int) -> None:
        self.header(screen, width, "C · Dual source")
        midpoint = width // 2
        panes = ((CUSTOM, 0, midpoint), (OFFICIAL, midpoint + 1, width - midpoint - 1))
        for mode, left, pane_width in panes:
            active = mode == self.mode
            style = curses.A_BOLD | (curses.color_pair(1) if active else curses.A_DIM)
            self.add(screen, 3, left + 1, "%s · %s" % (mode, self.sources[mode]), pane_width - 2, style)
            document_lines = markdown_lines(self.documents[mode]) if mode == CUSTOM else plain_lines(self.documents[mode])
            available = height - 6
            offset = min(self.scroll[mode], max(0, len(document_lines) - available))
            self.scroll[mode] = offset
            for row, (text, kind) in enumerate(document_lines[offset : offset + available], start=5):
                self.add(screen, row, left + 2, text, pane_width - 4, self.style(kind) if active else curses.A_DIM)
        for row in range(3, height - 1):
            self.add(screen, row, midpoint, "│", 1, curses.A_DIM)
        self.footer(screen, height, width)


def main() -> int:
    args = parse_args()
    if args.short:
        return short_output(args.command)
    prototype = Prototype(args.command)
    curses.wrapper(prototype.run)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
