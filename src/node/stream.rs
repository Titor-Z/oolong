use boa_engine::{Context, Module, Source};

fn stream_js_source() -> String {
    String::from(
        r#"
var _ev;
try { _ev = require("node:events"); } catch (e) {
  _ev = { EventEmitter: class {
    constructor() { this._listeners = {}; }
    on(n, fn) { (this._listeners[n] = this._listeners[n] || []).push(fn); return this; }
    off(n, fn) { const l = this._listeners[n]; if (l) { const i = l.indexOf(fn); if (i >= 0) l.splice(i, 1); } return this; }
    emit(n, ...a) { const l = this._listeners[n]; if (l) { for (const f of [...l]) f(...a); return true; } return false; }
    once(n, fn) { const g = (...a) => { this.off(n, g); fn(...a); }; this.on(n, g); return this; }
    addListener(n, fn) { return this.on(n, fn); }
    removeListener(n, fn) { return this.off(n, fn); }
  } };
}
var EventEmitter = _ev.EventEmitter;

var _destroy = function(err, cb) {
  this._destroyed = true;
  if (err) this.emit("error", err);
  cb && cb(err);
};

class Stream extends EventEmitter {
  constructor(opts) {
    super();
    this._destroyed = false;
  }

  pipe(dest, opts) {
    opts = opts || {};
    this.on("data", (data) => {
      const ok = dest.write(data);
      if (!ok) this.pause();
    });
    dest.on("drain", () => this.resume());
    this.on("end", () => { if (opts.end !== false) dest.end(); });
    this.on("error", (err) => dest.emit("error", err));
    dest.on("error", (err) => this.emit("error", err));
    return dest;
  }

  destroy(err) { _destroy.call(this, err); }
}

class Readable extends Stream {
  constructor(opts) {
    super(opts);
    opts = opts || {};
    this._readableState = {
      highWaterMark: opts.highWaterMark || 16384,
      buffer: [],
      flowing: null,
      ended: false,
      endedEmitted: false,
      reading: false,
      encoding: opts.encoding || null,
      objectMode: !!opts.objectMode,
    };
    this._read = opts.read || this._read;
  }

  on(event, fn) {
    var ret = super.on(event, fn);
    if (event === "data" && this._readableState.flowing !== false) {
      this.resume();
    }
    return ret;
  }

  addListener(event, fn) { return this.on(event, fn); }

  push(chunk, encoding) {
    var state = this._readableState;
    if (chunk === null) {
      state.ended = true;
      if (state.buffer.length === 0) queueMicrotask(() => this._emitEnd());
      return false;
    }
    if (typeof chunk === "string") chunk = Buffer.from(chunk, encoding || "utf8");
    if (!state.objectMode && (!chunk || chunk.length === 0)) return true;
    if (state.flowing) {
      this.emit("data", chunk);
      state.reading = false;
    } else {
      state.buffer.push(chunk);
    }
    return true;
  }

  _emitEnd() {
    var state = this._readableState;
    if (!state.endedEmitted) { state.endedEmitted = true; this.emit("end"); }
  }

  read(size) {
    var state = this._readableState;
    if (state.ended && state.buffer.length === 0) { this._emitEnd(); return null; }
    if (size && state.buffer.length > 0) {
      var chunks = []; var len = 0;
      while (state.buffer.length > 0 && len < size) {
        var c = state.buffer.shift(); chunks.push(c); len += c.length;
      }
      return chunks.length > 0 ? Buffer.concat(chunks) : null;
    }
    return state.buffer.shift() || null;
  }

  _read(size) {}

  resume() {
    var state = this._readableState;
    if (state.flowing !== true && !state.ended) { state.flowing = true; var _this = this; queueMicrotask(function() { _this._read(_this._readableState.highWaterMark); }); }
  }

  pause() { this._readableState.flowing = false; return this; }
  isPaused() { return this._readableState.flowing !== true; }

  pipe(dest, opts) { this.resume(); return Stream.prototype.pipe.call(this, dest, opts); }

  static from(data, opts) {
    var r = new Readable(opts);
    if (Array.isArray(data)) { for (const item of data) r.push(item); r.push(null); }
    else if (typeof data[Symbol.iterator] === "function") { for (const item of data) r.push(item); r.push(null); }
    else if (typeof data[Symbol.asyncIterator] === "function") {
      (async () => { for await (const item of data) r.push(item); r.push(null); })();
    } else { r.push(data); r.push(null); }
    return r;
  }

  destroy(err) { _destroy.call(this, err); this._emitEnd(); }
}

class Writable extends Stream {
  constructor(opts) {
    super(opts);
    opts = opts || {};
    this._writableState = {
      highWaterMark: opts.highWaterMark || 16384,
      writing: false, ended: false, buffer: [], corked: false,
      objectMode: !!opts.objectMode, decodeStrings: opts.decodeStrings !== false,
    };
    this._write = opts.write || this._write;
    this._final = opts.final || this._final;
  }

  write(chunk, encoding, cb) {
    var state = this._writableState;
    if (state.ended) {
      var err = new Error("write after end"); this.emit("error", err); cb && cb(err); return false;
    }
    if (typeof chunk === "string" && state.decodeStrings) chunk = Buffer.from(chunk, encoding || "utf8");
    if (typeof encoding === "function") { cb = encoding; encoding = null; }
    var needDrain = state.buffer.length >= state.highWaterMark;
    if (state.writing) { state.buffer.push({ chunk: chunk, cb: cb }); }
    else {
      state.writing = true;
      try {
        this._write(chunk, encoding || "buffer", (err) => {
          state.writing = false;
          if (err) { this.emit("error", err); cb && cb(err); return; }
          cb && cb();
          if (state.buffer.length > 0) { var item = state.buffer.shift(); this.write(item.chunk, item.cb); }
          if (state.ended && state.buffer.length === 0) { this._final(() => this.emit("finish")); }
        });
      } catch (e) { state.writing = false; this.emit("error", e); cb && cb(e); return false; }
    }
    return !needDrain;
  }

  _write(chunk, encoding, cb) { cb(); }
  _final(cb) { cb(); }

  end(chunk, encoding, cb) {
    var state = this._writableState;
    if (typeof chunk === "function") { cb = chunk; chunk = null; }
    else if (typeof encoding === "function") { cb = encoding; encoding = null; }
    state.ended = true;
    if (chunk !== null && chunk !== undefined) {
      this.write(chunk, encoding, () => { this._final(() => { this.emit("finish"); cb && cb(); }); });
    } else { this._final(() => { this.emit("finish"); cb && cb(); }); }
    return this;
  }

  cork() { this._writableState.corked = true; }
  uncork() {
    this._writableState.corked = false;
    var state = this._writableState;
    while (state.buffer.length > 0) { var item = state.buffer.shift(); this.write(item.chunk, item.cb); }
  }

  destroy(err) { _destroy.call(this, err); }
}

var _dk = Symbol("duplex");
class Duplex extends Readable {
  constructor(opts) {
    opts = opts || {};
    super(opts);
    this[_dk] = true;
    this._writableState = new Writable(opts)._writableState;
    this._write = opts.write || this._write || function(chunk, enc, cb) { cb(); };
    this._final = opts.final || this._final || function(cb) { cb(); };
  }

  write(chunk, encoding, cb) { return Writable.prototype.write.call(this, chunk, encoding, cb); }
  end(chunk, encoding, cb) { return Writable.prototype.end.call(this, chunk, encoding, cb); }
}

class Transform extends Duplex {
  constructor(opts) {
    opts = opts || {};
    super(opts);
    this._transform = opts.transform || this._transform;
    this._flush = opts.flush || this._flush;
  }

  _transform(chunk, encoding, cb) { this.push(chunk); cb(); }
  _flush(cb) { cb(); }

  _write(chunk, encoding, cb) {
    try { this._transform(chunk, encoding, cb); } catch (e) { cb(e); }
  }

  _final(cb) {
    var _this = this;
    this._flush(function(err) {
      if (err) { _this.emit("error", err); cb(err); return; }
      _this.push(null);
      cb();
    });
  }
}

class PassThrough extends Transform {
  _transform(chunk, encoding, cb) { this.push(chunk); cb(); }
}

function _pipeline(...args) {
  var cb = typeof args[args.length - 1] === "function" ? args.pop() : null;
  var streams = args;
  if (streams.length < 2) {
    var err = new Error("pipeline requires at least 2 streams");
    if (cb) queueMicrotask(() => cb(err)); else throw err;
    return;
  }

  function onError(err) { cleanup(); cb && cb(err); }
  function cleanup() { for (const s of streams) { s.removeListener("error", onError); } }

  for (const s of streams) s.on("error", onError);
  for (let j = 0; j < streams.length - 1; j++) streams[j].pipe(streams[j + 1]);

  var last = streams[streams.length - 1];
  var done = false;
  last.on("finish", () => { if (!done) { done = true; cleanup(); cb && cb(); } });
  for (const s of streams) {
    s.on("close", () => { if (s === last) { if (!done) { done = true; cleanup(); cb && cb(); } } });
  }
}

function _finished(stream, cb) {
  var done = false;
  function onEnd() { if (!done) { done = true; cleanup(); cb && cb(null); } }
  function onError(err) { if (!done) { done = true; cleanup(); cb && cb(err); } }
  function onFinish() { if (!done) { done = true; cleanup(); cb && cb(null); } }
  function cleanup() {
    stream.removeListener("end", onEnd);
    stream.removeListener("error", onError);
    stream.removeListener("finish", onFinish);
  }
  stream.on("end", onEnd); stream.on("error", onError); stream.on("finish", onFinish);
  return function () { done = true; cleanup(); };
}

var _stream = {
  Stream: Stream, Readable: Readable, Writable: Writable,
  Duplex: Duplex, Transform: Transform, PassThrough: PassThrough,
  pipeline: _pipeline, finished: _finished,
};

export { Stream, Readable, Writable, Duplex, Transform, PassThrough, _pipeline as pipeline, _finished as finished };
export default _stream;
"#,
    )
}

pub fn create_node_stream_module(context: &mut Context) -> Result<Module, String> {
    let js = stream_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:stream 模块失败: {e}"))
}
