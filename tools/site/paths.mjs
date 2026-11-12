// Locations shared by the site settings and the check that runs on the built site.
import { fileURLToPath } from 'node:url';

export const REPOSITORY_URL = 'https://github.com/tareksanger/SpaRs';

// GitHub Pages serves a project site under the repository's name.
export const BASE = `/${REPOSITORY_URL.split('/').at(-1)}/`;

export const DOCS_DIRECTORY = fileURLToPath(new URL('../../docs/', import.meta.url));
export const SITE_DIRECTORY = fileURLToPath(new URL('../../target/site/', import.meta.url));
