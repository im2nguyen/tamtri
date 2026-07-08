export function decodeBase64(dataBase64: string): Uint8Array {
  if (typeof atob === "function") {
    const binary = atob(dataBase64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }
  throw new Error("Base64 decode is unavailable in this environment.");
}

export function encodeBase64(bytes: Uint8Array): string {
  if (typeof btoa === "function") {
    let binary = "";
    bytes.forEach((byte) => {
      binary += String.fromCharCode(byte);
    });
    return btoa(binary);
  }
  throw new Error("Base64 encode is unavailable in this environment.");
}

export function bytesToText(bytes: Uint8Array): string {
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}
