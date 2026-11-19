# Demo frontend

Vite 8 + React 19 + TypeScript 7 app for the i18n pipeline example.
See [`../README.md`](../README.md) for the pipeline and the root
[`README.md`](../../README.md) for the extractor itself.

- `src/i18n.ts` — typed bindings to Django's `/jsi18n/` catalog functions
- `src/components/Cart.tsx` — `ngettext` / `pgettext` / `npgettext` + translator comments in JSX
- `vite.config.ts` — dev-server proxy so `npm run dev` can fetch `/jsi18n/` from Django on :8000
