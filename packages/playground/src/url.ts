import * as z from "zod/v4-mini";

export const urlPayloadSchema = z.object({
  source: z.optional(z.string()),
});
export type UrlPayload = z.infer<typeof urlPayloadSchema>;

export async function encodeUrlPayload(payload: UrlPayload): Promise<string> {
  const payloadData = new TextEncoder().encode(JSON.stringify(payload));
  const compressor = new CompressionStream("deflate-raw");
  const compressedStream = new Blob([payloadData]).stream().pipeThrough(compressor);
  const bytes = await new Response(compressedStream).bytes();
  const encoded = encodeBase64Url(Array.from(bytes, (b) => String.fromCharCode(b)).join(""));
  return "a" + encoded;
}

export async function decodeUrlPayload(encoded: string): Promise<UrlPayload | null> {
  if (!encoded.length) {
    return null;
  }

  const version = encoded.charAt(0);
  switch (version) {
    case "a":
      return decodeUrlPayloadV1(encoded.slice(1));
    default:
      throw new Error(`invalid payload version: ${version}`);
  }
}

async function decodeUrlPayloadV1(encoded: string): Promise<UrlPayload> {
  const compressed = Uint8Array.from(decodeBase64Url(encoded), (c) => c.charCodeAt(0));
  const decompressor = new DecompressionStream("deflate-raw");
  const decompressedStream = new Blob([compressed]).stream().pipeThrough(decompressor);
  return urlPayloadSchema.parse(await new Response(decompressedStream).json());
}

function encodeBase64Url(s: string): string {
  return btoa(s).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function decodeBase64Url(s: string): string {
  return atob(s.replaceAll("-", "+").replaceAll("_", "/"));
}
