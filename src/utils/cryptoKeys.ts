import { pbkdf2 } from '@noble/hashes/pbkdf2.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { bytesToHex } from '@noble/hashes/utils.js';
import { generateMnemonic, mnemonicToSeedSync, validateMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';

/**
 * Generate a 24-word BIP39 mnemonic phrase (256-bit entropy).
 */
export function generateMnemonicPhrase(): string {
  return generateMnemonic(wordlist, 256);
}

/**
 * Validate a BIP39 mnemonic phrase.
 */
export function validateMnemonicPhrase(mnemonic: string): boolean {
  return validateMnemonic(mnemonic, wordlist);
}

/**
 * Derive a 256-bit AES encryption key from a mnemonic phrase.
 * Uses BIP39 seed derivation followed by PBKDF2-SHA256.
 * Returns the key as a hex string.
 */
export function deriveAesKeyFromMnemonic(mnemonic: string): string {
  // Get the BIP39 seed (512-bit) from the mnemonic
  const seed = mnemonicToSeedSync(mnemonic);

  // Derive a 256-bit AES key using PBKDF2 with the seed
  const salt = new TextEncoder().encode('alphahuman-aes-key-v1');
  const derivedKey = pbkdf2(sha256, seed, salt, { c: 100000, dkLen: 32 });

  return bytesToHex(derivedKey);
}
