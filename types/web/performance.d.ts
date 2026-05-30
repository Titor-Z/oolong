declare class Performance {
  now(): DOMHighResTimeStamp
  mark(name?: string): void
  measure(name: string, startMark?: string, endMark?: string): void
  clear(): void
}
type DOMHighResTimeStamp = number
declare var performance: Performance
