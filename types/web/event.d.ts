declare class Event {
  constructor(type: string, options?: EventInit)
  readonly type: string
  readonly target: EventTarget | null
  readonly currentTarget: EventTarget | null
  readonly defaultPrevented: boolean
  readonly cancelable: boolean
  readonly bubbles: boolean
  readonly phase: number
  preventDefault(): void
  stopPropagation(): void
  stopImmediatePropagation(): void
}
declare class EventTarget {
  constructor()
  addEventListener(type: string, listener: EventListener | null, options?: boolean | AddEventListenerOptions): void
  removeEventListener(type: string, listener: EventListener | null, options?: boolean | EventListenerOptions): void
  dispatchEvent(event: Event): boolean
}
type EventListener = (event: Event) => void
interface EventInit { bubbles?: boolean; cancelable?: boolean; composed?: boolean }
interface AddEventListenerOptions { once?: boolean; passive?: boolean; capture?: boolean }
interface EventListenerOptions { capture?: boolean }
