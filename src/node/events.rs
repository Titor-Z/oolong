use boa_engine::{Context, Module, Source};

fn events_js_source() -> String {
    // In Node.js, defaultMaxListeners is a static property on EventEmitter
    // and each emitter instance has its own maxListeners (inherited from prototype).
    // We store _events as a Map<eventName, Set<{listener, wrapper}>>.
    r#"
let _defaultMaxListeners = 10;

class EventEmitter {
  constructor() {
    this._events = new Map();
    this._maxListeners = _defaultMaxListeners;
  }

  _addListener(eventName, listener, prepend, once) {
    if (typeof listener !== "function")
      throw new TypeError("listener must be a function");
    if (eventName === undefined || eventName === null)
      throw new TypeError("eventName must be a string or symbol");

    // Emit 'newListener' *before* adding
    if (eventName !== "newListener" && this._events.has("newListener")) {
      this.emit("newListener", eventName, listener);
    }

    let set = this._events.get(eventName);
    if (!set) {
      set = [];
      this._events.set(eventName, set);
    }

    const entry = { listener };
    if (once) {
      const wrapper = function(...args) {
        this.removeListener(eventName, wrapper);
        listener.apply(this, args);
      };
      wrapper.listener = listener;
      entry.listener = wrapper;
    }

    if (prepend) {
      set.unshift(entry);
    } else {
      set.push(entry);
    }

    // Warn if max listeners exceeded
    if (set.length > this._maxListeners && !set._warned) {
      set._warned = true;
      const msg = `Possible EventEmitter memory leak detected. ${set.length} ${String(eventName)} listeners added. Use emitter.setMaxListeners() to increase limit`;
      if (typeof console !== "undefined") console.warn(msg);
    }

    return this;
  }

  on(eventName, listener) {
    return this._addListener(eventName, listener, false, false);
  }

  addListener(eventName, listener) {
    return this._addListener(eventName, listener, false, false);
  }

  once(eventName, listener) {
    return this._addListener(eventName, listener, false, true);
  }

  prependListener(eventName, listener) {
    return this._addListener(eventName, listener, true, false);
  }

  prependOnceListener(eventName, listener) {
    return this._addListener(eventName, listener, true, true);
  }

  emit(eventName, ...args) {
    const set = this._events.get(eventName);
    if (!set || set.length === 0) return false;
    // Copy to avoid mutation during iteration
    const copy = [...set];
    for (const entry of copy) {
      entry.listener.apply(this, args);
    }
    return true;
  }

  removeListener(eventName, listener) {
    const set = this._events.get(eventName);
    if (!set || set.length === 0) return this;
    const idx = set.findIndex(
      e => e.listener === listener || (e.listener.listener === listener)
    );
    if (idx !== -1) {
      set.splice(idx, 1);
      if (set.length === 0) this._events.delete(eventName);
    }
    if (eventName !== "removeListener" && this._events.has("removeListener")) {
      this.emit("removeListener", eventName, listener);
    }
    return this;
  }

  off(eventName, listener) {
    return this.removeListener(eventName, listener);
  }

  removeAllListeners(eventName) {
    if (eventName === undefined) {
      this._events = new Map();
      return this;
    }
    this._events.delete(eventName);
    return this;
  }

  listeners(eventName) {
    const set = this._events.get(eventName);
    if (!set) return [];
    return set.map(e => e.listener.listener || e.listener);
  }

  rawListeners(eventName) {
    const set = this._events.get(eventName);
    if (!set) return [];
    return set.map(e => e.listener);
  }

  listenerCount(eventName) {
    const set = this._events.get(eventName);
    return set ? set.length : 0;
  }

  eventNames() {
    return [...this._events.keys()];
  }

  getMaxListeners() {
    return this._maxListeners;
  }

  setMaxListeners(n) {
    if (typeof n !== "number" || n < 0)
      throw new TypeError("n must be a non-negative number");
    this._maxListeners = n;
    return this;
  }

  static get defaultMaxListeners() {
    return _defaultMaxListeners;
  }

  static set defaultMaxListeners(val) {
    _defaultMaxListeners = val;
  }

  static listenerCount(emitter, eventName) {
    return emitter.listenerCount(eventName);
  }

  static async once(emitter, eventName) {
    return new Promise((resolve, reject) => {
      const handler = (...args) => resolve(args.length === 1 ? args[0] : args);
      emitter.once(eventName, handler);
    });
  }
}

export default EventEmitter;
export { EventEmitter };
"#.to_string()
}

pub fn create_node_events_module(context: &mut Context) -> Result<Module, String> {
    let js = events_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:events 模块失败: {e}"))
}
