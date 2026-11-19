import { gettext, ngettext, pgettext, npgettext, t } from "./i18n";

// Translators: title of the checkout screen
export const CHECKOUT_TITLE = gettext("Checkout");

// A plain comment that is not tagged and must not be attached
export const SLOGAN = gettext("Fresh produce, fast delivery");

export function CartBadge({ count }: { count: number }) {
  // Translators: items in the shopping cart, %(count)s is the number
  const label = ngettext("1 item", "%(count)s items", count);
  const flavor = npgettext(
    "ice cream",
    "%(count)s scoop",
    "%(count)s scoops",
    count,
  );
  return (
    <span aria-label={pgettext("cart", "Shopping cart")}>
      {label} · {flavor}
    </span>
  );
}

export const HELP_TEXT = t(
  `Multi-line
template literal`,
);

export const JOINED = _("Chunks are " + 'joined ' + `together`);

export const CASTED = gettext("typed away" as string);

export const TRICKY = gettext("Adjacent");

/* Translators: block comment
   spanning multiple lines */
export const BLOCK = gettext("Blocky");

export const DYNAMIC = gettext(buildLabel(user));
export const OPTS = t("key.with.opts", { ns: "common" });
const ignored = translate("not a configured function");
export const TRAILING = gettext("Trailing"); // Translators: trailing comment
export const AFTER_TRAILING = gettext("Still works");

export function Jsx() {
  return (
    <p>
      {/* Translators: idiomatic JSX translator comment */}
      {t("Brace wrapped comment")}
    </p>
  );
}
