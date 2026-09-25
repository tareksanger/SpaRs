#!/usr/bin/env node
import { downloadModel } from '../index.js';

/** @param {string[]} args @returns {Promise<void>} */
async function main(args) {
  const command = args.shift();
  if (command === '--help' || command === '-h' || command === 'help') {
    console.log('spars download MODEL [--path DIR] [--version VERSION] [--archive WHEEL]');
    return;
  }
  if (command !== 'download') throw new Error('Use `spars download MODEL [--path DIR]`.');
  const name = args.shift();
  if (!name || name.startsWith('-')) throw new Error('download requires a model name');
  /** @type {import('../index.js').DownloadOptions} */
  const options = {};
  while (args.length) {
    const flag = args.shift();
    const key = flag === '--path' ? 'path' : flag === '--archive' ? 'archive' : flag === '--version' ? 'version' : undefined;
    const value = args.shift();
    if (!key || !value || Object.hasOwn(options, key)) throw new Error('Unknown, duplicate, or missing option');
    options[key] = value;
  }
  console.log(await downloadModel(name, options));
}
main(process.argv.slice(2)).catch(error => { console.error(error.message); process.exitCode = 1; });
