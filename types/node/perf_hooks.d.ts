declare module "node:perf_hooks" {
  export function performanceNow(): number
  export function performanceMark(name?: string): void
  export function performanceMeasure(name: string, startMark?: string, endMark?: string): void
  export function performanceClear(): void
  export const performance: { now: typeof performanceNow; mark: typeof performanceMark; measure: typeof performanceMeasure; clear: typeof performanceClear }
  export default performance
}
