declare module "node:crypto" {
  export function randomBytes(size: number, callback?: (err: Error | null, buf: Buffer) => void): Buffer
  export function randomFillSync(buffer: Buffer, offset?: number, size?: number): Buffer
  export function randomUUID(): string
  export function createHash(algorithm: string): Hash
  export class Hash {
    update(data: string | Buffer, encoding?: string): this
    digest(encoding?: string): string | Buffer
  }
  export function createHmac(algorithm: string, key: string | Buffer): Hmac
  export class Hmac {
    update(data: string | Buffer, encoding?: string): this
    digest(encoding?: string): string | Buffer
  }
  export function createCipheriv(algorithm: string, key: Buffer, iv: Buffer | null): Cipher
  export class Cipher {
    update(data: Buffer, inputEncoding?: string, outputEncoding?: string): Buffer | string
    final(outputEncoding?: string): Buffer | string
  }
  export function createDecipheriv(algorithm: string, key: Buffer, iv: Buffer | null): Decipher
  export class Decipher {
    update(data: Buffer, inputEncoding?: string, outputEncoding?: string): Buffer | string
    final(outputEncoding?: string): Buffer | string
  }
  export const constants: Record<string, number>
}
