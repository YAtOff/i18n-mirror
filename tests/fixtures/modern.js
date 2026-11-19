// Modern ES2023+ that xgettext's JS parser chokes on — but tree-sitter eats it.
const i18n = { t: (s) => window.gettext(s) };

class Dashboard extends HTMLElement {
  #count = 0;
  static observedAttributes = ["count"];

  #render() {
    // Translators: heading on the dashboard card
    const heading = i18n.t("Dashboard overview");
    const tip = `Press ${"?"} for help`;
    return { heading, tip };
  }
}

const handler = async ({ detail: { items } }) => {
  const n = items?.length ?? 0;
  // Translators: n is the number of selected products
  return window.ngettext("%(n)s product selected", "%(n)s products selected", n);
};

const nested = i18n.utils?.t?.("Deeply optional");
const quoted = _("He said \"hello\"");
const single = _('it\'s single-quoted');
const unicode = gettext("Цената е %(price)s лв.");
