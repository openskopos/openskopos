// The entry point for Bunny Edge Scripting. `npm run build` bundles it with
// the WebAssembly module and the specification into one file,
// dist/bunny.js, and makes dist/node.js, the same server for Node, where the
// Bunny SDK listens on $PORT (default 8080).
import * as BunnySDK from "@bunny.net/edgescript-sdk";
import process from "node:process";
import * as sk from "@openskopos/core";
import wasmBase64 from "@openskopos/core/skopos_bg.wasm";
import specText from "../../spec/0.2/README.md";
import iconLight from "../../brand/avatar-512-light.png";
import iconDark from "../../brand/avatar-512-dark.png";
import iconSvg from "../../brand/favicon.svg";
import favicon from "../assets/favicon.ico";
import { createHandler } from "./server.js";

// Bunny allows 500 ms to start a script. Decoding and compiling the module
// takes about a quarter of that, and the first check another 300 ms to build
// the schema validator, so both happen on the first request instead.
let handler;

function start() {
  const wasm = typeof Uint8Array.fromBase64 === "function"
    ? Uint8Array.fromBase64(wasmBase64)
    : Uint8Array.from(atob(wasmBase64), (c) => c.charCodeAt(0));
  sk.initSync({ module: wasm });
  sk.check({});
  return createHandler({
    sk,
    specText,
    images: {
      "/icon-light.png": { type: "image/png", body: iconLight },
      "/icon-dark.png": { type: "image/png", body: iconDark },
      "/icon.svg": { type: "image/svg+xml", body: iconSvg },
      "/favicon.ico": { type: "image/x-icon", body: favicon },
    },
    env: () => process.env,
    healthy() {
      try {
        sk.version();
        sk.check({});
        return true;
      } catch {
        return false;
      }
    },
  });
}

BunnySDK.net.http.serve(
  { port: Number(process.env.PORT ?? 8080), hostname: process.env.HOST ?? "127.0.0.1" },
  (request) => (handler ??= start())(request),
);
