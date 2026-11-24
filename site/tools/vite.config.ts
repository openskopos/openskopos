// The build of openskopos.org/tools/. site/build.sh runs it and writes the
// pages into the tools folder of the site. Every page is a static HTML file;
// the shell below adds the same header to each of them.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";

const root = import.meta.dirname;
const tools = ["validate", "author", "compare", "history", "agent-file"];

// Replaces <!-- header --> in each page with the header of the site, and
// links the stylesheet of the site. In development, the server also serves
// the files of the site outside /tools/, such as /style.css.
function shell(): Plugin {
  const header = readFileSync(resolve(root, "src/header.html"), "utf8");
  const site = resolve(root, "..");
  return {
    name: "openskopos-shell",
    transformIndexHtml: {
      order: "post",
      handler: (html) =>
        html
          .replace("<!-- header -->", header)
          .replace('<meta charset="utf-8">', '<meta charset="utf-8">\n<link rel="stylesheet" href="/style.css">'),
    },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const path = (req.url ?? "").split("?")[0];
        if (!/^\/[\w.-]+\.(css|svg|png)$/.test(path)) return next();
        try {
          const body = readFileSync(resolve(site, "." + path));
          res.setHeader("Content-Type", path.endsWith(".css") ? "text/css" : path.endsWith(".svg") ? "image/svg+xml" : "image/png");
          res.end(body);
        } catch {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  root,
  base: "/tools/",
  plugins: [shell()],
  oxc: { jsx: { runtime: "automatic", importSource: "preact" } },
  optimizeDeps: { exclude: ["@openskopos/core"] },
  server: {
    fs: { allow: [resolve(root, "../..")] },
  },
  build: {
    outDir: resolve(root, "../../_site/tools"),
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: Object.fromEntries([
        ["index", resolve(root, "index.html")],
        ...tools.map((t) => [t, resolve(root, t, "index.html")]),
      ]),
    },
  },
});
