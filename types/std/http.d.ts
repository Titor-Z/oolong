declare module "@std/http" {
  /**
   * serve 函数选项
   */
  interface ServeOptions {
    /** 监听端口 */
    port: number
    /** 监听地址（默认 "0.0.0.0"） */
    hostname?: string
    /** 请求处理函数（接收 web Request 对象，返回 Response） */
    handler: (req: Request) => Response | Promise<Response>
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
