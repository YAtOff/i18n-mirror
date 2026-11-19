/**
 * Typed access to the translation catalog Django serves at /jsi18n/.
 *
 * The view is `django.views.i18n.JavaScriptCatalog` (see
 * https://docs.djangoproject.com/en/6.1/topics/i18n/translation/#translating-javascript-strings).
 * It defines these global functions from locale/<lang>/LC_MESSAGES/djangojs.mo:
 *
 *   gettext(msgid)
 *   ngettext(singular, plural, count)
 *   pgettext(context, msgid)
 *   npgettext(context, singular, plural, count)
 *   gettext_noop(msgid)
 *   interpolate(format, obj, named)
 *   get_format(format_type)
 *   pluralidx(count)
 *
 * The i18n-mirror extractor normalizes every wrapper below to one of the four
 * canonical call shapes, so using aliases costs nothing at extraction time.
 */

export {};

declare global {
  interface Window {
    gettext(msgid: string): string;
    ngettext(singular: string, plural: string, count: number): string;
    pgettext(context: string, msgid: string): string;
    npgettext(context: string, singular: string, plural: string, count: number): string;
    gettext_noop(msgid: string): string;
    interpolate(format: string, obj: Record<string, unknown>, named?: boolean): string;
    get_format(formatType: string): string;
    pluralidx(count: number | null): number;
  }
}

/**
 * Thin typed bindings to the catalog functions.
 *
 * Note these are function *references*, not wrappers: there is no call site
 * here for the extractor to misread — every `t("…")` in the components is the
 * call site that ends up in the .po file.
 */
export const t: (msgid: string) => string = window.gettext;
export const nt: (singular: string, plural: string, count: number) => string =
  window.ngettext;
export const pt: (context: string, msgid: string) => string = window.pgettext;
export const npt: (
  context: string,
  singular: string,
  plural: string,
  count: number,
) => string = window.npgettext;

/** Named interpolation: fmt("%(count)s of %(total)s", {count, total}, true). */
export const fill = (format: string, values: Record<string, unknown>): string =>
  window.interpolate(format, values, true);

export const i18n = { t, nt, pt, npt, fill };
