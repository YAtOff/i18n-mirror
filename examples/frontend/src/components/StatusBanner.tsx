import { t } from "../i18n.ts";

/**
 * Renders the delivery status banner.
 *
 * The long literal below exercises multi-line strings and em dashes; the
 * extractor re-emits it as one double-quoted literal in the mirror file.
 */
export default function StatusBanner() {
  const hour = new Date().getHours();
  if (hour < 9 || hour >= 21) {
    return (
      <div className="banner warning">
        {t("The market is closed — deliveries resume at 09:00")}
      </div>
    );
  }
  return (
    <div className="banner ok">
      {t("Catalog loaded successfully — the pipeline works.")}
    </div>
  );
}
