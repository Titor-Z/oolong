declare class EventEmitter {
  constructor();

  // Instance methods
  on(eventName: string | symbol, listener: (...args: any[]) => void): this;
  addListener(eventName: string | symbol, listener: (...args: any[]) => void): this;
  once(eventName: string | symbol, listener: (...args: any[]) => void): this;
  prependListener(eventName: string | symbol, listener: (...args: any[]) => void): this;
  prependOnceListener(eventName: string | symbol, listener: (...args: any[]) => void): this;

  emit(eventName: string | symbol, ...args: any[]): boolean;

  removeListener(eventName: string | symbol, listener: (...args: any[]) => void): this;
  off(eventName: string | symbol, listener: (...args: any[]) => void): this;
  removeAllListeners(eventName?: string | symbol): this;

  listeners(eventName: string | symbol): Function[];
  rawListeners(eventName: string | symbol): Function[];
  listenerCount(eventName: string | symbol): number;
  eventNames(): (string | symbol)[];

  getMaxListeners(): number;
  setMaxListeners(n: number): this;

  // Static properties
  static defaultMaxListeners: number;

  // Static methods
  static listenerCount(emitter: EventEmitter, eventName: string | symbol): number;
  static once(emitter: EventEmitter, eventName: string | symbol): Promise<any[]>;
}

export default EventEmitter;
export { EventEmitter };
