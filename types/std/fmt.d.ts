declare module "@std/fmt" {
  export namespace colors {
    export function red(text: string): string
    export function green(text: string): string
    export function yellow(text: string): string
    export function blue(text: string): string
    export function magenta(text: string): string
    export function cyan(text: string): string
    export function white(text: string): string
    export function gray(text: string): string
    export function bold(text: string): string
    export function dim(text: string): string
    export function italic(text: string): string
    export function underline(text: string): string
    export function stripColor(text: string): string
  }

  export function sprintf(format: string, ...args: any[]): string

  const _default: {
    colors: typeof colors
    sprintf: typeof sprintf
  }
  export default _default
}
