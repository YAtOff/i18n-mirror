import Cart from "./components/Cart.tsx";
import ProductList from "./components/ProductList.tsx";
import StatusBanner from "./components/StatusBanner.tsx";
import { t } from "./i18n.ts";

export default function App() {
  return (
    <main className="app">
      <h1>{t("Fresh market — order online")}</h1>
      <StatusBanner />
      <ProductList />
      <Cart />
    </main>
  );
}
