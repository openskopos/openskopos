// Share links: the text goes into the URL fragment, compressed. Browsers never
// send the fragment to a server, so a shared document stays between the
// people who have the link.

async function pipe(bytes: Uint8Array, stream: CompressionStream | DecompressionStream): Promise<Uint8Array> {
  const out = new Blob([bytes as BlobPart]).stream().pipeThrough(stream);
  return new Uint8Array(await new Response(out).arrayBuffer());
}

const toBase64Url = (b: Uint8Array) => btoa(Array.from(b, (c) => String.fromCharCode(c)).join("")).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const fromBase64Url = (s: string) => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));

export async function shareUrl(value: unknown): Promise<string> {
  const packed = await pipe(new TextEncoder().encode(JSON.stringify(value)), new CompressionStream("deflate-raw"));
  const url = new URL(location.href);
  url.hash = "s=" + toBase64Url(packed);
  return url.toString();
}

/** The shared value in the fragment of this page, if there is one. Removes the fragment. */
export async function readShared<T>(): Promise<T | undefined> {
  const m = /^#s=([\w-]+)$/.exec(location.hash);
  if (!m) return undefined;
  history.replaceState(null, "", location.pathname + location.search);
  try {
    const bytes = await pipe(fromBase64Url(m[1]), new DecompressionStream("deflate-raw"));
    return JSON.parse(new TextDecoder().decode(bytes)) as T;
  } catch {
    return undefined;
  }
}
