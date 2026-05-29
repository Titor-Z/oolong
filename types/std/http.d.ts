declare module "@std/http" {
  /**
   * HTTP 请求对象（由 serve handler 接收）
   */
  interface ServeRequest {
    readonly method: string
    readonly url: string
    readonly headers: Record<string, string>
    readonly body?: string
  }

  /**
   * serve 函数选项
   */
  interface ServeOptions {
    /** 监听端口 */
    port: number
    /** 监听地址（默认 "0.0.0.0"） */
    hostname?: string
    /** 请求处理函数 */
    handler: (req: ServeRequest) => Response | Promise<Response>
  }

  /**
   * 启动 HTTP 服务器
   *
   * 阻塞当前执行，处理传入的 HTTP 请求。
   *
   * @example
   * ```ts
   * import { serve } from "@std/http"
   *
   * serve({
   *   port: 3000,
   *   handler: (req) => {
   *     return new Response("Hello World")
   *   },
   * })
   * ```
   */
  export function serve(options: ServeOptions): never

  /**
   * serve 的命名空间导出（兼容 `import http from "@std/http"`）
   */
  export default { serve: typeof serve }
}
