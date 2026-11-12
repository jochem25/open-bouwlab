// Lokale stand-in voor Caddy + Authentik forward-auth (alleen voor lokaal testen).
//
// Luistert UITSLUITEND op 127.0.0.1:<poort>, verwijdert binnenkomende
// X-Authentik-*-headers en zet een vaste lokale gebruiker met de opgegeven
// groepen, en stuurt door naar de API op 127.0.0.1:<api-poort>. Zo werken de
// entitlement-modules (trap, constructie) lokaal zonder Authentik.
// Niet voor productie: in productie zet Caddy/Authentik deze headers.
//
// Gebruik: node scripts/lokaal-auth-proxy.mjs <poort> <api-poort> [groepen]
//   groepen: komma-gescheiden, standaard "openbouwlab-trap"
import http from "node:http";

const HOST = "127.0.0.1";
const poort = Number(process.argv[2] ?? 3011);
const apiPoort = Number(process.argv[3] ?? poort + 1);
const groepen = (process.argv[4] ?? "openbouwlab-trap").split(",").map((g) => g.trim()).filter(Boolean);
const GEBRUIKER = "lokaal";

const server = http.createServer((req, res) => {
  const headers = {};
  for (const [k, v] of Object.entries(req.headers)) {
    if (!k.toLowerCase().startsWith("x-authentik-")) headers[k] = v;
  }
  headers["x-authentik-username"] = GEBRUIKER;
  headers["x-authentik-name"] = "Lokale gebruiker";
  headers["x-authentik-groups"] = groepen.join("|");
  headers.host = `${HOST}:${apiPoort}`;
  const doel = http.request(
    { host: HOST, port: apiPoort, method: req.method, path: req.url, headers },
    (antwoord) => {
      res.writeHead(antwoord.statusCode ?? 502, antwoord.headers);
      antwoord.pipe(res);
    },
  );
  doel.on("error", (e) => {
    res.writeHead(502, { "content-type": "text/plain" });
    res.end(`API niet bereikbaar op ${HOST}:${apiPoort}: ${e.message}`);
  });
  req.pipe(doel);
});

server.listen(poort, HOST, () => {
  console.log(`lokale auth-proxy: http://localhost:${poort} -> API ${apiPoort}, groepen ${groepen.join(", ")}`);
});
