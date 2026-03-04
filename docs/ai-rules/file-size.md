# File Size Rule

**Max file size: 300 lines.** 500 lines maximum with a comment at the top justifying the exception.

## Rules

- One main idea per file. If a file does two things, split it.
- `mod.rs` files orchestrate and re-export — they do NOT contain all domain logic.
- Complex domains always use a subdirectory with per-concern files:
  - `tui/views/` (not one giant layout.rs)
  - `tui/events/` (not one giant app.rs)
  - `live/` domain files (not one giant live/mod.rs)
- No stub files. If a file has no logic, delete it.

## Enforcement

Before writing any file, estimate line count. If >300 lines, split into multiple files first.

When modifying an existing file that exceeds 300 lines, split it as part of the change.
