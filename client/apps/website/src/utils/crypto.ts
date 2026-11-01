// End-to-End Encryption (E2EE) Utility using Web Crypto AES-256-GCM

/**
 * Generate a random 256-bit symmetric key and return it as a hex string.
 * This key is intended to be stored in the URL fragment (#key) so it is
 * never sent over HTTP/WebSocket requests to the server.
 */
export function generateSessionKey(): string {
  const bytes = new Uint8Array(32);
  window.crypto.getRandomValues(bytes);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Convert a hex string key into a CryptoKey for AES-GCM
 */
async function importKeyFromHex(hexKey: string): Promise<CryptoKey> {
  const match = hexKey.match(/.{1,2}/g);
  if (!/^[a-f0-9]{64}$/i.test(hexKey) || !match || match.length !== 32) {
    throw new Error(
      "Invalid session encryption key length. Expected 256 bits (64 hex characters).",
    );
  }
  const bytes = new Uint8Array(match.map((byte) => parseInt(byte, 16)));
  return await window.crypto.subtle.importKey(
    "raw",
    bytes,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

/**
 * Encrypts plaintext using AES-256-GCM with a fresh 12-byte IV.
 * Returns base64 encoded ciphertext and IV.
 */
export async function encryptMessage(
  plaintext: string,
  rawHexKey: string,
): Promise<{ ciphertext: string; iv: string }> {
  const key = await importKeyFromHex(rawHexKey);
  const iv = new Uint8Array(12);
  window.crypto.getRandomValues(iv);

  const encoder = new TextEncoder();
  const encodedPlaintext = encoder.encode(plaintext);

  const encryptedBuffer = await window.crypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv,
    },
    key,
    encodedPlaintext,
  );

  const ciphertextBase64 = btoa(String.fromCharCode(...new Uint8Array(encryptedBuffer)));
  const ivBase64 = btoa(String.fromCharCode(...iv));

  return { ciphertext: ciphertextBase64, iv: ivBase64 };
}

/**
 * Decrypts AES-256-GCM ciphertext using the base64 IV and the raw hex key.
 */
export async function decryptMessage(
  ciphertextBase64: string,
  ivBase64: string,
  rawHexKey: string,
): Promise<string> {
  try {
    const key = await importKeyFromHex(rawHexKey);

    const ciphertextBytes = Uint8Array.from(atob(ciphertextBase64), (c) => c.charCodeAt(0));
    const ivBytes = Uint8Array.from(atob(ivBase64), (c) => c.charCodeAt(0));

    const decryptedBuffer = await window.crypto.subtle.decrypt(
      {
        name: "AES-GCM",
        iv: ivBytes,
      },
      key,
      ciphertextBytes,
    );

    const decoder = new TextDecoder();
    return decoder.decode(decryptedBuffer);
  } catch (err) {
    console.error("Failed to decrypt message:", err);
    return "[Encrypted Message - Unable to Decrypt. Check Session Key]";
  }
}
