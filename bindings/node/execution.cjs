// @ts-check
'use strict';

/** @typedef {import('./index.js').ExecutionOptions} ExecutionOptions */
/** @typedef {{ start: () => void, next: Job | null }} Job */

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
  maxActive = 2;
  maxQueued = 32;

  /** @param {ExecutionOptions} options */
  configure(options) {
    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new TypeError('Expected execution options');
    const { maxActive, maxQueued } = options;
    if (!Number.isSafeInteger(maxActive) || maxActive < 1 || !Number.isSafeInteger(maxQueued) || maxQueued < 0) {
      throw new RangeError('maxActive must be a positive safe integer; maxQueued must be a nonnegative safe integer');
    }
    if (this.active || this.queued) throw new Error('Configure execution only while inference is idle');
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
/** @param {ExecutionOptions} options */
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
  installed.add(prototype);
  const process = prototype.process;
  const batch = prototype.processBatch;
  prototype.process = function(text, stage) {
    if (!(this instanceof binding.Model)) throw new TypeError('Expected a Model receiver');
    if (typeof text !== 'string') throw new TypeError('Expected a text string');
    validateStage(stage);
    return scheduler.submit(() => process.call(this, text, stage));
  };
  prototype.processBatch = function(texts, stage) {
    if (!(this instanceof binding.Model)) throw new TypeError('Expected a Model receiver');
    if (!Array.isArray(texts)) throw new TypeError('Expected an array of text strings');
    validateStage(stage);
    if (scheduler.full()) return busy();
    // Capture strings at submission, including for jobs that wait in JavaScript.
    const count = texts.length;
    /** @type {string[]} */
    const snapshot = [];
    for (let i = 0; i < count; i++) {
      const text = texts[i];
      if (typeof text !== 'string') throw new TypeError('Expected a text string');
      snapshot.push(text);
    }
    return scheduler.submit(() => batch.call(this, snapshot, stage));
  };
}

module.exports = { Scheduler, configureExecution, install };
