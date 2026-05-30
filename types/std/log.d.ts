declare module "@std/log" {
  export enum LogLevel {
    DEBUG = 10,
    INFO = 20,
    WARN = 30,
    ERROR = 40,
    FATAL = 50,
  }

  export interface LogSetup {
    format?: "text" | "json"
    colors?: Partial<Record<"debug" | "info" | "warn" | "error" | "fatal", string>>
    formatter?: (record: LogRecord) => string
  }

  export interface LogRecord {
    level: LogLevel
    levelName: string
    msg: string
    args: unknown[]
    loggerName: string
    timestamp: Date
  }

  export interface LoggerOptions {
    level?: LogLevel
    format?: "text" | "json"
    colors?: Partial<Record<"debug" | "info" | "warn" | "error" | "fatal", string>>
    formatter?: (record: LogRecord) => string
  }

  export class Logger {
    readonly name: string
    readonly level: LogLevel

    debug(msg: string, ...args: unknown[]): void
    info(msg: string, ...args: unknown[]): void
    warn(msg: string, ...args: unknown[]): void
    error(msg: string, ...args: unknown[]): void
    fatal(msg: string, ...args: unknown[]): void

    child(bindings: Record<string, unknown>): Logger
  }

  export function debug(msg: string, ...args: unknown[]): void
  export function info(msg: string, ...args: unknown[]): void
  export function warn(msg: string, ...args: unknown[]): void
  export function error(msg: string, ...args: unknown[]): void
  export function fatal(msg: string, ...args: unknown[]): void
  export function child(bindings: Record<string, unknown>): Logger

  export function getLogger(name?: string): Logger

  export function setup(options: LogSetup): void

  const _default: {
    Logger: typeof Logger
    LogLevel: typeof LogLevel
    getLogger: typeof getLogger
    setup: typeof setup
    debug: typeof debug
    info: typeof info
    warn: typeof warn
    error: typeof error
    fatal: typeof fatal
  }
  export default _default
}
