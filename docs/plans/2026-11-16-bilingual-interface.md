# Bilingual Interface and Documentation Implementation Plan

**Goal:** Improve the ledger interface, add Chinese and English display languages, and publish illustrated installation and usage documentation in v0.2.0.

**Architecture:** Keep the existing public SDK and financial calculations. Use a small reactive translation module for interface labels, public UI component locales, backend message presentation and numeric formatting. Keep financial data, stored snapshots and Excel schema unchanged. A responsive sidebar, clear page headers, metric cards and accessible CSS bar charts improve the overview without additional chart dependencies.

**Tech Stack:** Vue 3, TypeScript, public Codex Proxy UI, SVG, CSS, Rust, Playwright for browser verification.

## Design

An ink and jade ledger identity: a compact book/chart SVG mark, dark navigation, calm content surfaces, generous spacing and tabular figures. Host light/dark themes remain supported. No remote fonts or runtime assets. On narrow screens navigation wraps above content. All chart values come from the current report; no fabricated trends or comparisons.

Language changes update labels, form fields, table columns, notices and chart descriptions without changing user-entered names. Host sandbox restrictions mean storage may be unavailable: the language selector must work in memory, and persistence is best effort. Existing English/Chinese user data stays intact. Excel remains the established Chinese seven-sheet workbook, explicitly documented.

## Steps

1. Add `frontend/src/i18n.ts`, `components/AppIcon.vue` and bilingual entity field definitions. Verify English UI strings and component locale switching.
2. Refine `App.vue`, `GroupTable.vue`, `EntityEditor.vue` and styles; preserve all current management routes and actions, add setup guidance and an in-app workflow guide.
3. Add original SVG icon and repository banner; expose the icon through the plugin's existing resource declaration and management page metadata.
4. Write Chinese and English README, installation and usage guides with a consistent worked example, CSV instructions, download behavior and honest limitations.
5. Run typecheck/lint/build and Rust checks; exercise both languages, themes, narrow screens, exports, edits and snapshots in the sandbox test host. Capture real fixture screenshots labelled as demo data.
6. Bump version to 0.2.0, build Linux x64, verify artifacts, publish source and a new GitHub Release without altering v0.1.0 assets.

## Verification

- Switching language preserves report, draft inputs, user-entered names and current navigation.
- English pages contain English navigation, control labels and translated operational feedback; built-in UI dialogs use the selected locale.
- Charts expose exact request counts, show empty/zero states, cap the display with an explicit remainder group and do not sum currencies.
- Installation instructions match the pinned host's actual upload, validation and trust steps; documentation distinguishes native plugin upload from browser extension installation and source archives.
- Screenshots come from the actual compiled application with fictional fixtures and are explicitly labelled.
