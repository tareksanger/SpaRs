// Check the built documentation site: run after `vitepress build tools/site`.
//
// The build itself fails on a link to a guide that does not exist. This checks what the build
// does not: links to anything else inside the site, links that still point outside it, the tab
// titles, the version menu, and the files written for language models.
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, posix } from 'node:path';
import { BASE, SITE_DIRECTORY, SITE_VERSION, SITE_VERSIONS, siteVersionUrl } from './paths.mjs';

// The copy named by SPARS_SITE_VERSION is the one checked; unset checks the root copy.
const COPY = `${SITE_VERSION ?? 'root'} copy`;
const problems = [];

function read(name) {
  const path = join(SITE_DIRECTORY, name);
  if (!existsSync(path) || statSync(path).size === 0) {
    problems.push(`${name} is missing or empty; build the site first`);
    return '';
  }
  return readFileSync(path, 'utf8');
}

function existsInSite(sitePath) {
  const path = join(SITE_DIRECTORY, decodeURIComponent(sitePath));
  return [path, `${path}.html`, join(path, 'index.html')].some((candidate) => existsSync(candidate) && statSync(candidate).isFile());
}

const pages = readdirSync(SITE_DIRECTORY).filter((name) => name.endsWith('.html'));
for (const page of pages) {
  for (const [, href] of read(page).matchAll(/href="([^"]*)"/g)) {
    const target = href.split('#')[0].split('?')[0];
    if (target === '' || /^[a-z][a-z0-9+.-]*:/i.test(href)) continue;
    // Pages sit together at the top of the site, so a relative link starts from there.
    const sitePath = target.startsWith(BASE) ? target.slice(BASE.length) : posix.normalize(target);
    if (sitePath.startsWith('/') || sitePath.startsWith('..')) {
      problems.push(`${page}: link "${href}" leaves the site without naming a full address`);
    } else if (!existsInSite(sitePath)) {
      problems.push(`${page}: link "${href}" points at a file the site does not contain`);
    }
  }
}

const home = read('index.html');
for (const title of ['Rust', 'Node.js']) {
  if (!home.includes(`data-title="${title}"`)) problems.push(`index.html: no code tab titled "${title}"`);
}
for (const version of SITE_VERSIONS) {
  if (!home.includes(`href="${siteVersionUrl(version)}"`)) problems.push(`index.html: the version menu has no link to ${version}`);
}

read('llms.txt');
for (const name of ['llms-full.txt', 'index.md']) {
  const leaving = read(name).match(/\]\(\.\.\/[^)]*\)/g) ?? [];
  if (leaving.length > 0) problems.push(`${name}: ${leaving.length} links leave the site, such as ${leaving[0]}`);
}

if (problems.length > 0) {
  console.error(problems.map((problem) => `${COPY}: ${problem}`).join('\n'));
  process.exit(1);
}
console.log(`Checked ${pages.length} pages of the ${COPY}: links stay inside the site, tabs are titled, llms files are present.`);
