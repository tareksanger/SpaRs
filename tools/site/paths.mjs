// Locations shared by the site settings and the check that runs on the built site.
import { fileURLToPath } from 'node:url';

export const REPOSITORY_URL = 'https://github.com/tareksanger/SpaRs';

// A copy of the site is published for `main` under "next" and for each release under its tag.
export const NEXT_VERSION = 'next';
const RELEASE_TAG = /^v\d+\.\d+\.\d+$/;

export function isReleaseTag(name) {
  return RELEASE_TAG.test(name);
}

function siteVersionName(name, variable) {
  if (name !== NEXT_VERSION && !isReleaseTag(name)) {
    throw new Error(`${variable} has "${name}"; use "next", a release tag such as "v1.2.3", or leave it unset.`);
  }
  return name;
}

// The copy being built, named by SPARS_SITE_VERSION. Unset builds the copy served at the site root.
const versionName = process.env.SPARS_SITE_VERSION;
export const SITE_VERSION = versionName === undefined ? undefined : siteVersionName(versionName, 'SPARS_SITE_VERSION');

// Every published copy, in menu order, as a comma-separated SPARS_SITE_VERSIONS. Unset publishes
// the root copy alone, with no version menu.
const versionNames = process.env.SPARS_SITE_VERSIONS;
export const SITE_VERSIONS =
  versionNames === undefined ? [] : versionNames.split(',').map((name) => siteVersionName(name, 'SPARS_SITE_VERSIONS'));

const VERSION_FOLDER = SITE_VERSION === undefined ? '' : `${SITE_VERSION}/`;

// GitHub Pages serves a project site at <owner>.github.io under the repository's name; a
// versioned copy sits one folder below.
const [OWNER, REPOSITORY] = new URL(REPOSITORY_URL).pathname.split('/').filter(Boolean);
const ROOT = `/${REPOSITORY}/`;
export const BASE = `${ROOT}${VERSION_FOLDER}`;

// The full address of a published copy. Links between copies need it, because a link that
// starts with "/" is taken to be inside the copy that holds it.
export function siteVersionUrl(version) {
  return `https://${OWNER.toLowerCase()}.github.io${ROOT}${version}/`;
}

export const REPOSITORY_DIRECTORY = fileURLToPath(new URL('../../', import.meta.url));
export const DOCS_DIRECTORY = fileURLToPath(new URL('../../docs/', import.meta.url));
export const SITE_DIRECTORY = fileURLToPath(new URL(`../../target/site/${VERSION_FOLDER}`, import.meta.url));

// Outside every copy's folder, so the build cache is never published.
export const SITE_CACHE_DIRECTORY = fileURLToPath(new URL('../../target/site-cache/', import.meta.url));
