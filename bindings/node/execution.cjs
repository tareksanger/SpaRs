// @ts-check
'use strict';

/** @typedef {import('./index.js').ExecutionOptions} ExecutionOptions */
/** @typedef {import('./index.js').InputLimits} InputLimits */
/** @typedef {{ start: () => void, next: Job | null }} Job */

const DEFAULT_MAX_ACTIVE = 2;
const DEFAULT_MAX_QUEUED = 32;

/** A bounded FIFO that submits only active jobs to the native worker pool. */
class Scheduler {
  /** @type {number} */
  active = 0;
  /** @type {number} */
  queued = 0;
  /** @type {Job | null} */
  head = null;
  /** @type {Job | null} */
  tail = null;
  maxActive = DEFAULT_MAX_ACTIVE;
  maxQueued = DEFAULT_MAX_QUEUED;

  assertIdle() {
    if (this.active || this.queued) throw new Error('Configure processing only while inference is idle');
  }

  /** @param {ExecutionOptions | null} options */
  configure(options) {
    if (options === null) {
      this.assertIdle();
      this.maxActive = DEFAULT_MAX_ACTIVE;
      this.maxQueued = DEFAULT_MAX_QUEUED;
      return;
    }
    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new TypeError('Expected execution options');
    const { maxActive, maxQueued } = options;
    if (!Number.isSafeInteger(maxActive) || maxActive < 1 || !Number.isSafeInteger(maxQueued) || maxQueued < 0) {
      throw new RangeError('maxActive must be a positive safe integer; maxQueued must be a nonnegative safe integer');
    }
    this.assertIdle();
    this.maxActive = maxActive;
    this.maxQueued = maxQueued;
  }

  full() { return this.active >= this.maxActive && this.queued >= this.maxQueued; }

  /** @template T @param {() => Promise<T>} run @returns {Promise<T>} */
  submit(run) {
    if (this.full()) {
      return busy();
    }
    return new Promise((resolve, reject) => {
      const start = () => {
        this.active++;
        // A synchronous native failure must also return its admission slot.
        try {
          run().then(value => { this.finish(); resolve(value); }, error => { this.finish(); reject(error); });
        } catch (error) { this.finish(); reject(error); }
      };
      if (this.active < this.maxActive) start();
      else {
        const job = { start, next: null };
        if (this.tail) this.tail.next = job;
        else this.head = job;
        this.tail = job;
        this.queued++;
      }
    });
  }

  finish() {
    this.active--;
    const job = this.head;
    if (job) {
      this.head = job.next;
      if (!this.head) this.tail = null;
      this.queued--;
      job.start();
    }
  }
}

/** @returns {Promise<never>} */
function busy() { return Promise.reject(Object.assign(new Error('Inference queue is full'), { code: 'SPARS_BUSY' })); }

const scheduler = new Scheduler();
/** @type {InputLimits | null} */
let inputLimits = null;

/** @param {InputLimits | null} options */
function configureInputLimits(options) {
  if (options === null) { scheduler.assertIdle(); inputLimits = null; return; }
  if (!options || typeof options !== 'object' || Array.isArray(options)) throw new TypeError('Expected input limits');
  const { maxTextLength, maxBatchSize, maxBatchTextLength } = options;
  if (![maxTextLength, maxBatchSize, maxBatchTextLength].every(value => Number.isSafeInteger(value) && value > 0)) {
    throw new RangeError('Input limits must be positive safe integers');
  }
  scheduler.assertIdle();
  inputLimits = { maxTextLength, maxBatchSize, maxBatchTextLength };
}

/** @param {string} message @returns {Promise<never>} */
function tooLarge(message) {
  return Promise.reject(Object.assign(new Error(message), { code: 'SPARS_INPUT_LIMIT' }));
}

/** @param {ExecutionOptions | null} options */
function configureExecution(options) { scheduler.configure(options); }

/** @type {WeakSet<object>} */
const installed = new WeakSet();

/** @param {unknown} stage */
function validateStage(stage) {
  if (stage != null && (typeof stage !== 'string' || !['Tokenizer', 'Tagger', 'Parser', 'AttributeRuler', 'Lemmatizer', 'Ner'].includes(stage))) {
    throw new TypeError('Unknown pipeline stage');
  }
}

/** @param {{ Model: typeof import('./index.js').Model }} binding */
function install(binding) {
  const prototype = binding.Model.prototype;
  if (installed.has(prototype)) return;
  // An older override can load successfully but lacks settings needed for chunking.
  // Validate every required accessor before changing any native method or descriptor.
  for (const name of ['maxLength', 'batchSize']) {
    const descriptor = Object.getOwnPropertyDescriptor(prototype, name);
    if (typeof descriptor?.get !== 'function' || typeof descriptor?.set !== 'function') {
      throw Object.assign(new Error('Native addon is incompatible with this @spars/node wrapper. Reinstall matching package versions and remove or update NAPI_RS_NATIVE_LIBRARY_PATH.'), {
        code: 'SPARS_NATIVE_INCOMPATIBLE',
      });
    }
  }
  installed.add(prototype);
  // Freeze pipeline settings during submitted work, including JS-queued jobs.
  for (const name of ['maxLength', 'batchSize']) {
    const descriptor = Object.getOwnPropertyDescriptor(prototype, name);
    const set = descriptor?.set;
    if (descriptor && set) Object.defineProperty(prototype, name, {
      ...descriptor,
      set(value) { scheduler.assertIdle(); set.call(this, value); },
    });
  }
  const process = prototype.process;
  const batch = prototype.processBatch;
  prototype.process = function(text, stage) {
    if (!(this instanceof binding.Model)) throw new TypeError('Expected a Model receiver');
    if (typeof text !== 'string') throw new TypeError('Expected a text string');
    validateStage(stage);
    if (inputLimits && text.length > inputLimits.maxTextLength) return tooLarge('Text exceeds maxTextLength (UTF-16 units)');
    return scheduler.submit(() => process.call(this, text, stage));
  };
  prototype.processBatch = function(texts, stage) {
    if (!(this instanceof binding.Model)) throw new TypeError('Expected a Model receiver');
    if (!Array.isArray(texts)) throw new TypeError('Expected an array of text strings');
    validateStage(stage);
    const limits = inputLimits;
    const size = this.batchSize;
    const count = texts.length;
    if (limits && count > limits.maxBatchSize) return tooLarge('Batch exceeds maxBatchSize');
    if (scheduler.full()) return busy();
    // Capture strings at submission, including for jobs that wait in JavaScript.
    let remaining = limits?.maxBatchTextLength ?? Infinity;
    /** @type {string[]} */
    const snapshot = [];
    for (let i = 0; i < count; i++) {
      const text = texts[i];
      if (typeof text !== 'string') throw new TypeError('Expected a text string');
      if (limits && text.length > limits.maxTextLength) return tooLarge('Batch text exceeds maxTextLength (UTF-16 units)');
      if (text.length > remaining) return tooLarge('Batch exceeds maxBatchTextLength (UTF-16 units)');
      remaining -= text.length;
      snapshot.push(text);
    }
    return scheduler.submit(async () => {
      /** @type {import('./index.js').Document[]} */
      const documents = [];
      for (let offset = 0; offset < snapshot.length; offset += size) {
        const result = await batch.call(this, snapshot.slice(offset, offset + size), stage);
        for (const doc of result) documents.push(doc);
      }
      return documents;
    });
  };
  prototype.pipe = async function*(texts, options = {}) {
    if (!(this instanceof binding.Model)) throw new TypeError('Expected a Model receiver');
    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new TypeError('Expected pipe options');
    for (const key of Object.keys(options)) {
      if (key !== 'batchSize' && key !== 'stage') throw new TypeError(`Unsupported pipe option: ${key}`);
    }
    const size = options.batchSize ?? this.batchSize;
    const stage = options.stage;
    if (!Number.isInteger(size) || size < 1 || size > 0xffffffff) throw new RangeError('batchSize must be a positive u32 integer');
    validateStage(stage);
    /** @type {string[]} */
    let buffer = [];
    for await (const text of texts) {
      if (typeof text !== 'string') throw new TypeError('Expected a text string');
      buffer.push(text);
      if (buffer.length === size) {
        const docs = await this.processBatch(buffer, stage);
        buffer = [];
        yield* docs;
      }
    }
    if (buffer.length) yield* await this.processBatch(buffer, stage);
  };
}

module.exports = { Scheduler, configureExecution, configureInputLimits, install };
