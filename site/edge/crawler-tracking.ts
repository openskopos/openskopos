// Bunny Edge Script (middleware) for the openskopos.org pull zone. It reports
// requests from AI assistants, search engines and training crawlers to
// DataFast, which shows them in the Bot traffic card. Crawlers do not run the
// browser script on the pages, so this runs on the CDN instead.
//
// The pull zone needs "Run script before cache" turned on. Without it the
// client hooks below never run. site/edge/README.md has the whole setup.

import * as BunnySDK from "npm:@bunny.net/edgescript-sdk@0.13.0";
import { trackAICrawlerResponse } from "npm:@datafast/ai-crawl@1.0.9";

const config = {
  // The public website ID, the same one as in the script tag on each page.
  websiteId: "dfid_1r4Qm5UNUqHW9qbB0ZomE",
  domain: "openskopos.org",
  publicOrigin: "https://openskopos.org",
  // The Bot traffic token (dfbot_...), from a secret of the script in Bunny.
  // Never put it in this file. Without it, DataFast accepts the requests until
  // "Reject unauthenticated requests" is turned on.
  authToken: Deno.env.get("DATAFAST_BOT_TOKEN"),
  // Bunny replaces any X-Real-IP that the visitor sends. The package would
  // first read headers such as CF-Connecting-IP, which a visitor can set.
  getIp: (request: Request) => request.headers.get("x-real-ip"),
};

declare const Bunny: { v1: { waitUntil(promise: Promise<unknown>): void } };

BunnySDK.net.http
  // The URL is used only by local runs. On Bunny, the pull zone's origin is used.
  .servePullZone({ url: "https://openskopos.org" })
  .onClientResponse(async (ctx) => {
    // The package skips people and static files, and sends crawler requests in
    // the background, so the response never waits for DataFast.
    trackAICrawlerResponse(ctx.request, ctx.response, {
      waitUntil: (promise) => Bunny.v1.waitUntil(promise),
    }, config);
    return ctx.response;
  });
