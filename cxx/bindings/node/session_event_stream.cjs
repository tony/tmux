"use strict";

/** @typedef {import("tmux-cxx").SessionEventStreamItem} SessionEventStreamItem */
/** @typedef {import("tmux-cxx").SessionEventStreamOptions} SessionEventStreamOptions */

/**
 * Describe the native session stream after installing its JavaScript lifecycle facade.
 * @typedef {import("tmux-cxx").SessionEventStream & {
 *   readNext: (timeoutMs?: number) => Promise<SessionEventStreamItem | null>
 *   closeNative: () => void
 * }} NativeSessionEventStream
 */

/**
 * Carry validated stream wait settings.
 * @typedef NormalizedStreamOptions
 * @property {number} timeoutMs - Native wait deadline in milliseconds.
 * @property {AbortSignal | undefined} signal - Caller cancellation signal, when supplied.
 */

/**
 * Validate one public stream wait before crossing the native boundary.
 * @param {SessionEventStreamOptions | undefined} options - Wait deadline and cancellation signal.
 * @returns {NormalizedStreamOptions} Validated settings with the default deadline applied.
 */
function streamOptions(options) {
  if (options === undefined) return { timeoutMs: 500, signal: undefined };
  if (options === null || typeof options !== "object") {
    throw new TypeError("stream options must be an object");
  }
  const timeoutMs = options.timeoutMs ?? 500;
  if (!Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 750) {
    throw new RangeError("timeoutMs must be an integer from 1 through 750");
  }
  if (options.signal !== undefined && !(options.signal instanceof AbortSignal)) {
    throw new TypeError("signal must be an AbortSignal");
  }
  return { timeoutMs, signal: options.signal };
}

/**
 * Convert an AbortSignal reason into one stable JavaScript error.
 * @param {AbortSignal} signal - Signal whose completed cancellation supplies the reason.
 * @returns {Error} The caller's Error reason or a standard AbortError.
 */
function abortError(signal) {
  const reason = /** @type {unknown} */ (signal.reason);
  return reason instanceof Error
    ? reason
    : new DOMException("The operation was aborted", "AbortError");
}

/**
 * Recognize native closure outcomes while an async iterator is ending.
 * @param {unknown} error - Rejection raised by the native stream wait.
 * @returns {boolean} Whether closure or cancellation ended the stream.
 */
function isClosedStreamError(error) {
  if (!(error instanceof Error)) return false;
  const code = /** @type {unknown} */ (Reflect.get(error, "code"));
  return code === "cancelled" || code === "closed";
}

/**
 * Install lifecycle and async-iteration methods on the addon's session stream type.
 * @param {typeof import("tmux-cxx") & { SessionEventStream: { prototype: NativeSessionEventStream } }} native - Loaded native addon whose stream prototype is extended once.
 */
function installSessionEventStream(native) {
  const streamPrototype = native.SessionEventStream.prototype;
  const nativeReadNext = streamPrototype.readNext;
  const nativeClose = streamPrototype.closeNative;
  const closedStreams = new WeakSet();

  /**
   * Mark this JavaScript stream closed before cancelling native observation.
   * @this {NativeSessionEventStream}
   */
  function close() {
    closedStreams.add(this);
    nativeClose.call(this);
  }

  /**
   * Await one item without blocking JavaScript and bind cancellation to this stream.
   * @this {NativeSessionEventStream}
   * @param {SessionEventStreamOptions} [options] - Wait deadline and cancellation signal.
   */
  async function next(options) {
    const { timeoutMs, signal } = streamOptions(options);
    if (signal?.aborted) {
      this.close();
      throw abortError(signal);
    }
    const stream = this;
    /** Close this stream when its caller aborts the admitted wait. */
    function aborted() {
      stream.close();
    }
    signal?.addEventListener("abort", aborted, { once: true });
    try {
      return await nativeReadNext.call(this, timeoutMs);
    } catch (error) {
      if (signal?.aborted) throw abortError(signal);
      throw error;
    } finally {
      signal?.removeEventListener("abort", aborted);
    }
  }

  /**
   * Pull one bounded native wait at a time so consumer pace supplies backpressure.
   * @param {{ stream: NativeSessionEventStream, options: SessionEventStreamOptions | undefined }} iteration - Owned stream and per-pull wait settings.
   * @yields {SessionEventStreamItem} Each committed event or explicit recovery gap.
   */
  async function* iterateSessionEvents({ stream, options }) {
    while (!closedStreams.has(stream)) {
      try {
        // oxlint-disable-next-line no-await-in-loop -- One owned cursor forbids overlapping reads.
        const item = await stream.next(options);
        if (item !== null) yield item;
      } catch (error) {
        if (closedStreams.has(stream) && isClosedStreamError(error)) return;
        throw error;
      }
    }
  }

  /**
   * Iterate nonempty stream items until explicit closure or caller cancellation.
   * @this {NativeSessionEventStream}
   * @param {SessionEventStreamOptions} [options] - Wait deadline and cancellation signal.
   */
  function events(options) {
    return iterateSessionEvents({ stream: this, options });
  }

  /**
   * Iterate this stream with default timeout and cancellation settings.
   * @this {NativeSessionEventStream}
   * @returns {AsyncIterator<SessionEventStreamItem>} The stream's default event iterator.
   */
  function asyncIterator() {
    return this.events()[Symbol.asyncIterator]();
  }

  /**
   * Close this stream during `await using` disposal.
   * @this {NativeSessionEventStream}
   * @returns {Promise<void>} Completed disposal after native cancellation is requested.
   */
  function asyncDispose() {
    this.close();
    return Promise.resolve();
  }

  Object.defineProperties(streamPrototype, {
    close: { value: close },
    next: { value: next },
    events: { value: events },
    [Symbol.asyncIterator]: { value: asyncIterator },
    [Symbol.asyncDispose]: { value: asyncDispose },
  });
}

module.exports = { installSessionEventStream };
