import { useState } from "react";
import { npt, nt, pt, t } from "../i18n.ts";

interface CartProps {
  initialCount?: number;
}

export default function Cart({ initialCount = 3 }: CartProps) {
  const [count, setCount] = useState(initialCount);

  // Translators: summary of the shopping cart; %(count)s is the item count
  const summary = nt("1 item", "%(count)s items", count);

  return (
    <section className="cart">
      <h2>{pt("checkout", "Your shopping cart")}</h2>
      <p data-testid="cart-summary">{summary}</p>

      <button type="button" onClick={() => setCount(0)}>
        {/* Translators: label of the button that adds the item to the cart */}
        {t("Add to cart")}
      </button>

      <footer>
        <p>{pt("checkout", "Continue to checkout")}</p>
        <p>
          {npt(
            "checkout",
            "%(count)s step remaining",
            "%(count)s steps remaining",
            count,
          )}
        </p>
      </footer>
    </section>
  );
}
