declare module "node:zlib" {
  export function gzipSync(data: Buffer, options?: { level?: number }): Buffer
  export function gunzipSync(data: Buffer): Buffer
  export function deflateSync(data: Buffer, options?: { level?: number }): Buffer
  export function inflateSync(data: Buffer): Buffer
  export function brotliCompressSync(data: Buffer, options?: { params?: Record<number, number> }): Buffer
  export function brotliDecompressSync(data: Buffer): Buffer
  export const constants: { Z_OK: number; Z_ERRNO: number; Z_STREAM_ERROR: number; Z_DATA_ERROR: number; Z_MEM_ERROR: number; Z_BUF_ERROR: number; Z_VERSION_ERROR: number }
}
