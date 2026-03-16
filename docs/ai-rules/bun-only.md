# Bun Only

This project uses **bun** exclusively as the JavaScript package manager and script runner. Never use npm, npx, yarn, or pnpm.

| Instead of | Use |
|---|---|
| `npm install` | `bun install` |
| `npm ci` | `bun install --frozen-lockfile` |
| `npm run <script>` | `bun run <script>` |
| `npx <cmd>` | `bun <cmd>` |
| `package-lock.json` | `bun.lockb` |

This applies to: Makefile targets, `tauri.conf.json` commands, CI workflows, documentation, and any shell commands you suggest or write.
