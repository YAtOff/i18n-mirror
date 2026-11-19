# Real-world extraction fixtures

Unmodified source files from active open-source React/TypeScript projects,
fetched from each project's default branch on 2026-11-16. They exercise the
parser on modern syntax (decorators, generics, `import.meta`, tagged
templates, object-literal i18n APIs) the way real code does — not the way
hand-written fixtures do.

| file | project | license | what it exercises |
|---|---|---|---|
| `outline-settings.tsx` | [outline/outline](https://github.com/outline/outline) `app/actions/definitions/settings.tsx` | Apache-2.0 | dense `t("…")` calls, two calls on one line |
| `outline-api-keys.tsx` | [outline/outline](https://github.com/outline/outline) `app/scenes/Settings/ApiKeys.tsx` | Apache-2.0 | TSX scenes, hooks, generics |
| `excalidraw-app-main-menu.tsx` | [excalidraw/excalidraw](https://github.com/excalidraw/excalidraw) `excalidraw-app/components/AppMainMenu.tsx` | MIT | dropdown menus, icon props |
| `react-admin-settings-page.tsx` | [marmelab/react-admin](https://github.com/marmelab/react-admin) `examples/crm/src/settings/SettingsPage.tsx` | Apache-2.0 | `import.meta.env`, clipboard APIs; no literal calls → 0 extracted |
| `zulip-compose-closed-ui.ts` | [zulip/zulip](https://github.com/zulip/zulip) `web/src/compose_closed_ui.ts` | Apache-2.0 | 13 KB of TS with `$t({defaultMessage: …})` object-literal i18n — parses cleanly, extracts nothing (correct: not string-literal gettext calls) |
