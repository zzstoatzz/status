import { defineConfig } from "@hatk/hatk/config";

const isProd = process.env.NODE_ENV === "production";

const scopes = [
  "atproto",
  "repo:io.zzstoatzz.status.record",
  "repo:io.zzstoatzz.status.preferences",
].join(" ");

export default defineConfig({
  // our own relay. note that seq numbering is per-relay — switching hosts
  // invalidates any saved _cursor.relay, which must be cleared rather than
  // carried across.
  relay: isProd ? "wss://zlay.waow.tech" : "ws://localhost:2583",
  plc: isProd ? "https://plc.directory" : "http://localhost:2582",
  port: 3000,
  databaseEngine: "sqlite",
  database: isProd ? "/data/status.db" : "data/status.db",
  backfill: {
    signalCollections: ["io.zzstoatzz.status.record"],
    fullNetwork: false,
    parallelism: 2,
  },
  oauth: {
    issuer: isProd ? "https://status.zzstoatzz.io" : undefined,
    scopes: scopes.split(" "),
    clients: [
      ...(isProd
        ? [
            {
              client_id: "https://status.zzstoatzz.io/oauth-client-metadata.json",
              client_name: "status",
              scope: scopes,
              redirect_uris: ["https://status.zzstoatzz.io/oauth/callback"],
            },
          ]
        : []),
      {
        client_id: "http://127.0.0.1:3000/oauth-client-metadata.json",
        client_name: "status",
        scope: scopes,
        redirect_uris: ["http://127.0.0.1:3000/oauth/callback"],
      },
    ],
  },
});
