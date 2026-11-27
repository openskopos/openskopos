# Crawler tracking on the CDN

openskopos.org is built by GitHub Pages and served through a Bunny pull zone.
The script [crawler-tracking.ts](crawler-tracking.ts) runs on that pull zone.
It reports requests from AI assistants, search engines and training crawlers
to DataFast, which shows them in the Bot traffic card. See
[DataFast's documentation](https://datafa.st/docs/bot-traffic-tracking).

The pages already have the DataFast script for people. Crawlers do not run
JavaScript, so only something in front of the site can see them. The script
skips people and static files, and it sends crawler requests in the
background, so pages never wait for DataFast. DataFast checks the crawler's IP
address against the published ranges of each company.

DNS stays at DNSimple, and the site still deploys with the Deploy site
workflow.

## Set it up

1. In Bunny, add a pull zone on the Standard tier.
   - Origin URL: `http://185.199.108.153`, one of the GitHub Pages addresses.
   - Turn on Forward Host Header. GitHub Pages picks the site by its hostname.
     Without it, every page shows GitHub's "Site not found" page.

   Do not use `openskopos.github.io` as the origin. GitHub redirects it to
   openskopos.org, which goes back to Bunny and loops.
2. Under Hostnames, add `openskopos.org` and `www.openskopos.org`.
3. Under Edge Scripting, add a Middleware script. Paste in
   `crawler-tracking.ts` and connect the script to the pull zone.
4. In the pull zone, go to General > Origin and turn on Run script before
   cache. Without it the script runs only when the cache misses.
5. In DataFast, open the settings of the Bot traffic card and create a token.
   In Bunny, open the script, go to Env Configuration > Secrets and add the
   secret `DATAFAST_BOT_TOKEN` with the token. Publish the script again.
   When the check below works, turn on Reject unauthenticated requests in
   DataFast. Then nobody else can send fake crawler requests to the site's
   Bot traffic card.
6. In the repository settings, under Pages, turn off Enforce HTTPS. Bunny
   pulls from GitHub over HTTP, and GitHub would redirect it to HTTPS, which
   loops. Keep the custom domain `openskopos.org`.
7. In DNSimple, replace the records of the site:
   - `openskopos.org`: remove the A and AAAA records for GitHub Pages and add
     an ALIAS record to the pull zone's hostname, such as
     `openskopos.b-cdn.net`.
   - `www.openskopos.org`: change the CNAME record to the same hostname.
8. When DNS points to Bunny, go back to Hostnames. For each hostname, get the
   free certificate and turn on Force SSL. Bunny now redirects visitors to
   HTTPS.

## Check it

Send a request with a crawler's user agent:

```sh
curl -sI -A "Mozilla/5.0 (compatible; GPTBot/1.2; +https://openai.com/gptbot)" https://openskopos.org/llms.txt
```

The response has `server: BunnyCDN`. Within a minute, the Bot traffic card in
DataFast shows the request. The card shows only IP-verified crawlers by
default, so turn that filter off to see this test. If the request is missing,
read the script's logs in Bunny.

## Change the script

Edit `crawler-tracking.ts`, then paste it into the script in Bunny again.
Bunny does not deploy it from this repository, and `site/build.sh` leaves
this folder out of the site.

To run it on your machine with [Deno](https://deno.com), run
`deno run -A site/edge/crawler-tracking.ts`. It serves openskopos.org on
http://127.0.0.1:8080, but it sends nothing to DataFast, because the
background call works only on Bunny.

## Undo it

In DNSimple, remove the ALIAS record and put back the records for GitHub
Pages: A records for 185.199.108.153 to 185.199.111.153, and AAAA records for
2606:50c0:8000::153 to 2606:50c0:8003::153. Point
`www.openskopos.org` back to `openskopos.github.io`. Then turn Enforce HTTPS
back on in the Pages settings.
