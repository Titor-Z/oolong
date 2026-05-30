declare module "node:module" {
  export function createRequire(filename: string | URL): NodeRequire
  export interface NodeRequire {
    (id: string): any
    resolve(id: string, options?: { paths?: string[] }): string
  }
}
