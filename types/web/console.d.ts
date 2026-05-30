// https://console.spec.whatwg.org/
interface Console {
  log(...data: any[]): void
  debug(...data: any[]): void
  info(...data: any[]): void
  warn(...data: any[]): void
  error(...data: any[]): void
  trace(...data: any[]): void
  assert(condition?: boolean, ...data: any[]): void

  time(label?: string): void
  timeEnd(label?: string): void
  timeLog(label?: string, ...data: any[]): void

  count(label?: string): void
  countReset(label?: string): void

  clear(): void
  table(data?: any): void

  group(...data: any[]): void
  groupEnd(): void
  groupCollapsed(...data: any[]): void
}

declare var console: Console
