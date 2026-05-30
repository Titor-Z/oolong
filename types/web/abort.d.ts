declare class AbortController {
  constructor()
  readonly signal: AbortSignal
  abort(reason?: any): void
}
declare class AbortSignal extends EventTarget {
  readonly aborted: boolean
  readonly reason: any
  onabort: ((event: Event) => void) | null
  throwIfAborted(): void
  static abort(reason?: any): AbortSignal
  static timeout(ms: number): AbortSignal
}
