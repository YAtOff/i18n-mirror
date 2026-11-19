/**
 * End-to-end runtime verification of the i18n pipeline.
 *
 * Boots the Django dev server, fetches the JavaScriptCatalog (/jsi18n/) for
 * Bulgarian, executes the served catalog script in a sandbox exactly like a
 * browser would, and asserts the documented catalog functions
 * (gettext, ngettext, pgettext, npgettext, interpolate, pluralidx, get_format)
 * return the translations that were extracted from the React sources.
 *
 * Also checks the server-rendered page (django domain) in both languages.
 *
 * Usage: node tests/verify_catalog.mjs
 */
import { spawn } from "node:child_process";
import vm from "node:vm";

const HOST = "127.0.0.1";
const PORT = 8123;
const BASE = `http://${HOST}:${PORT}`;
const PY = ".venv/bin/python";

let failures = 0;
function check(name, actual, expected) {
  const ok = actual === expected;
  if (!ok) failures += 1;
  console.log(`${ok ? "  ✓" : "  ✗"} ${name}${ok ? "" : `\n      expected ${JSON.stringify(expected)}\n      actual   ${JSON.stringify(actual)}`}`);
}

async function waitForServer(url, timeoutMs = 20000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await fetch(url, { signal: AbortSignal.timeout(1000) });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 300));
    }
  }
  throw new Error("server did not come up");
}

const server = spawn(PY, ["manage.py", "runserver", `${HOST}:${PORT}`, "--noreload"], {
  stdio: ["ignore", "pipe", "pipe"],
});
const serverLog = [];
server.stdout.on("data", (d) => serverLog.push(d.toString()));
server.stderr.on("data", (d) => serverLog.push(d.toString()));

try {
  await waitForServer(`${BASE}/jsi18n/`);

  // --- djangojs catalog ---------------------------------------------------
  const res = await fetch(`${BASE}/jsi18n/`, {
    headers: { "Accept-Language": "bg" },
  });
  if (!res.ok) throw new Error(`/jsi18n/ returned ${res.status}`);
  const catalogJs = await res.text();

  // Execute the served script like a browser would: sloppy mode, global `this`.
  const sandbox = {};
  vm.createContext(sandbox);
  vm.runInContext(catalogJs, sandbox, { filename: "jsi18n.js" });

  console.log("djangojs catalog (Accept-Language: bg):");
  check(
    'gettext("Add to cart")',
    sandbox.gettext("Add to cart"),
    "Добави в количката",
  );
  check(
    'ngettext("1 item", "%(count)s items", 1)',
    sandbox.ngettext("1 item", "%(count)s items", 1),
    "1 артикул", // bg plural form 0
  );
  check(
    'ngettext("1 item", "%(count)s items", 3)',
    sandbox.ngettext("1 item", "%(count)s items", 3),
    "%(count)s артикула", // bg plural form 1
  );
  check(
    'pgettext("checkout", "Continue to checkout")',
    sandbox.pgettext("checkout", "Continue to checkout"),
    "Продължи към плащане",
  );
  check(
    "npgettext('checkout', steps, 5)",
    sandbox.npgettext(
      "checkout",
      "%(count)s step remaining",
      "%(count)s steps remaining",
      5,
    ),
    "Остават %(count)s стъпки",
  );
  check(
    'interpolate(ngettext(...items, 3), {count: 3}, true)',
    sandbox.interpolate(sandbox.ngettext("1 item", "%(count)s items", 3), { count: 3 }, true),
    "3 артикула",
  );
  check("pluralidx(1) (bg: nplurals=2)", sandbox.pluralidx(1), 0);
  check("pluralidx(0)", sandbox.pluralidx(0), 1);
  check(
    'gettext("The market is closed — deliveries resume at 09:00")',
    sandbox.gettext("The market is closed — deliveries resume at 09:00"),
    "Пазарът е затворен — доставките се възобновяват в 09:00",
  );
  check("untranslated msgid falls through", sandbox.gettext("no such msgid"), "no such msgid");

  // --- server-side page (django domain) ------------------------------------
  console.log("django domain (server-rendered page):");
  const bgPage = await (await fetch(BASE + "/", { headers: { "Accept-Language": "bg" } })).text();
  check("bg page shows translated title", bgPage.includes("Модерен React i18n конвейер за Django"), true);
  check("bg page loads the catalog script", bgPage.includes('src="/jsi18n/"'), true);

  const enPage = await (await fetch(BASE + "/", { headers: { "Accept-Language": "en" } })).text();
  check("en page shows English title", enPage.includes("Modern React i18n pipeline for Django"), true);

  console.log(failures === 0 ? "\nall checks passed — the pipeline works end to end" : `\n${failures} check(s) FAILED`);
} catch (err) {
  console.error("error:", err.message);
  console.error(serverLog.join(""));
  failures += 1;
} finally {
  server.kill("SIGTERM");
}

process.exit(failures === 0 ? 0 : 1);
