#!/usr/bin/env node
/**
 * CORS proxy for LM Studio → Canvas fetch from Cursor webview.
 * LM Studio API works from curl/terminal but blocks browser fetch without CORS headers.
 *
 * Usage: node scripts/lm-cors-proxy.mjs
 * Canvas default URL: http://127.0.0.1:14580/api/v1/chat
 */

import http from "node:http";

const TARGET_HOST = "127.0.0.1";
const TARGET_PORT = 1234;
const PROXY_PORT = 14580;

function setCors(res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization");
}

const server = http.createServer((req, res) => {
  setCors(res);

  if (req.method === "OPTIONS") {
    res.writeHead(204);
    res.end();
    return;
  }

  const bodyChunks = [];
  req.on("data", (chunk) => bodyChunks.push(chunk));
  req.on("end", () => {
    const body = Buffer.concat(bodyChunks);
    const path = req.url ?? "/";

    const upstream = http.request(
      {
        hostname: TARGET_HOST,
        port: TARGET_PORT,
        path,
        method: req.method,
        headers: {
          "Content-Type": req.headers["content-type"] ?? "application/json",
          "Content-Length": body.length,
        },
      },
      (upRes) => {
        const chunks = [];
        upRes.on("data", (c) => chunks.push(c));
        upRes.on("end", () => {
          const payload = Buffer.concat(chunks);
          setCors(res);
          res.writeHead(upRes.statusCode ?? 502, {
            "Content-Type": upRes.headers["content-type"] ?? "application/json",
          });
          res.end(payload);
        });
      },
    );

    upstream.on("error", (err) => {
      setCors(res);
      res.writeHead(502, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          error: `LM Studio unreachable at ${TARGET_HOST}:${TARGET_PORT}: ${err.message}`,
        }),
      );
    });

    upstream.write(body);
    upstream.end();
  });
});

server.listen(PROXY_PORT, "127.0.0.1", () => {
  console.log(`LM Studio CORS proxy: http://127.0.0.1:${PROXY_PORT} → http://${TARGET_HOST}:${TARGET_PORT}`);
  console.log("Keep this running while using the ASR Prompt Tester canvas.");
});
