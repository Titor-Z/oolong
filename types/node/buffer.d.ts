declare module "node:buffer" {
  export class Buffer extends Uint8Array {
    static from(data: ArrayLike<number> | ArrayBuffer | string, encoding?: BufferEncoding): Buffer
    static alloc(size: number, fill?: number | string, encoding?: BufferEncoding): Buffer
    static allocUnsafe(size: number): Buffer
    static byteLength(string: string | ArrayBuffer | DataView, encoding?: BufferEncoding): number
    static concat(list: Buffer[], totalLength?: number): Buffer
    static compare(buf1: Buffer, buf2: Buffer): number
    static isBuffer(obj: any): obj is Buffer
    static isEncoding(encoding: string): boolean

    constructor(data: ArrayLike<number> | ArrayBuffer | string, encoding?: BufferEncoding)

    length: number
    write(string: string, offset?: number, length?: number, encoding?: BufferEncoding): number
    toString(encoding?: BufferEncoding, start?: number, end?: number): string
    toJSON(): { type: "Buffer"; data: number[] }
    equals(other: Buffer): boolean
    compare(other: Buffer, targetStart?: number, targetEnd?: number, sourceStart?: number, sourceEnd?: number): number
    copy(target: Buffer, targetStart?: number, sourceStart?: number, sourceEnd?: number): number
    slice(start?: number, end?: number): Buffer
    subarray(start?: number, end?: number): Buffer
    fill(value: number | string | Buffer, offset?: number, end?: number, encoding?: BufferEncoding): this
    includes(value: string | number | Buffer, byteOffset?: number, encoding?: BufferEncoding): boolean
    indexOf(value: string | number | Buffer, byteOffset?: number, encoding?: BufferEncoding): number
    lastIndexOf(value: string | number | Buffer, byteOffset?: number, encoding?: BufferEncoding): number
    swap16(): Buffer
    swap32(): Buffer
    swap64(): Buffer
    readUint8(offset?: number): number
    readUint16BE(offset?: number): number
    readUint16LE(offset?: number): number
    readUint32BE(offset?: number): number
    readUint32LE(offset?: number): number
    readBigUint64BE(offset?: number): bigint
    readBigUint64LE(offset?: number): bigint
    readInt8(offset?: number): number
    readInt16BE(offset?: number): number
    readInt16LE(offset?: number): number
    readInt32BE(offset?: number): number
    readInt32LE(offset?: number): number
    readBigInt64BE(offset?: number): bigint
    readBigInt64LE(offset?: number): bigint
    readFloatBE(offset?: number): number
    readFloatLE(offset?: number): number
    readDoubleBE(offset?: number): number
    readDoubleLE(offset?: number): number
    writeUint8(value: number, offset?: number): number
    writeUint16BE(value: number, offset?: number): number
    writeUint16LE(value: number, offset?: number): number
    writeUint32BE(value: number, offset?: number): number
    writeUint32LE(value: number, offset?: number): number
    writeBigUint64BE(value: bigint, offset?: number): number
    writeBigUint64LE(value: bigint, offset?: number): number
    writeInt8(value: number, offset?: number): number
    writeInt16BE(value: number, offset?: number): number
    writeInt16LE(value: number, offset?: number): number
    writeInt32BE(value: number, offset?: number): number
    writeInt32LE(value: number, offset?: number): number
    writeBigInt64BE(value: bigint, offset?: number): number
    writeBigInt64LE(value: bigint, offset?: number): number
    writeFloatBE(value: number, offset?: number): number
    writeFloatLE(value: number, offset?: number): number
    writeDoubleBE(value: number, offset?: number): number
    writeDoubleLE(value: number, offset?: number): number
    entries(): IterableIterator<[number, number]>
    keys(): IterableIterator<number>
    values(): IterableIterator<number>
  }

  type BufferEncoding = "ascii" | "utf8" | "utf-8" | "utf16le" | "ucs2" | "ucs-2" | "base64" | "base64url" | "hex" | "latin1" | "binary"
  const SlowBuffer: typeof Buffer

  export { BufferEncoding }
}

declare var Buffer: import("node:buffer").Buffer
