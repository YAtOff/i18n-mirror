import { fill, t } from "../i18n.ts";

interface Product {
  name: string;
  price: number;
  unit: string;
  stock: number;
}

const PRODUCTS: Product[] = [
  { name: "Radenichki tomatoes", price: 4.5, unit: "kg", stock: 12 },
  { name: "Staroplaninski honey", price: 18.0, unit: "jar", stock: 3 },
  { name: "Banski starets", price: 22.0, unit: "kg", stock: 0 },
];

export default function ProductList() {
  return (
    <section className="products">
      <h2>{t("Today's market stall")}</h2>
      <ul>
        {PRODUCTS.map((p) => (
          <li key={p.name} className={p.stock === 0 ? "sold-out" : undefined}>
            <strong>{p.name}</strong>
            <span>
              {fill(t("%(price)s per %(unit)s"), {
                price: p.price.toFixed(2),
                unit: p.unit,
              })}
            </span>
            {p.stock === 0 ? (
              // Translators: shown when a product is out of stock
              <em>{t("Sold out — come back Friday")}</em>
            ) : (
              // Translators: %(stock)s is the remaining unit count
              <em>{fill(t("Only %(stock)s left"), { stock: p.stock })}</em>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
