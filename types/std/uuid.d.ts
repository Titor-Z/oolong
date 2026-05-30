declare module "@std/uuid" {
  export function v4(): string
  export function validate(uuid: string): boolean

  const _default: {
    v4: typeof v4
    validate: typeof validate
  }
  export default _default
}
