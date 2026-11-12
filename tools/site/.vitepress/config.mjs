import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { defineConfig } from 'vitepress';
import llmstxt from 'vitepress-plugin-llms';
import { BASE, DOCS_DIRECTORY, REPOSITORY_URL, SITE_DIRECTORY } from '../paths.mjs';

const VUE_PACKAGE_DIRECTORY = dirname(createRequire(import.meta.url).resolve('vue/package.json'));
const SPACY_URL = 'https://spacy.io';

// Guides link to repository files that are not site pages: files outside docs/ through `../`
// paths, and files inside docs/ that are not Markdown, such as the model schemas. Each such link
// opens the file on GitHub. Links to guides, to other sites and within a page are left alone.
function repositoryFileUrl(href) {
  const path = href.split('#')[0];
  if (path === '' || path.startsWith('/') || /^[a-z][a-z0-9+.-]*:/i.test(href)) return undefined;
  if (path.endsWith('.md') && !path.startsWith('../')) return undefined;
  if (!existsSync(join(DOCS_DIRECTORY, path))) return undefined;
  return new URL(`blob/main/docs/${href}`, `${REPOSITORY_URL}/`).href;
}

function linkRepositoryFiles(md) {
  md.core.ruler.push('repository_file_links', (state) => {
    for (const block of state.tokens) {
      for (const token of block.children ?? []) {
        const href = token.type === 'link_open' ? token.attrGet('href') : null;
        const url = href ? repositoryFileUrl(href) : undefined;
        if (url) token.attrSet('href', url);
      }
    }
  });
}

// The llms plugin copies the Markdown source, so the same links are rewritten in its output.
function linkRepositoryFilesInMarkdown(directory) {
  const names = readdirSync(directory).filter((name) => name.endsWith('.md') || name === 'llms-full.txt');
  for (const name of names) {
    const path = join(directory, name);
    const source = readFileSync(path, 'utf8');
    const linked = source.replace(/\]\(([^)\s]+)\)/g, (link, href) => {
      const url = repositoryFileUrl(href);
      return url ? `](${url})` : link;
    });
    if (linked !== source) writeFileSync(path, linked);
  }
}

// Runnable guide examples use a bare language fence, so a code group would title its tabs
// "rust" and "typescript". Each tab is titled for the reader instead.
const TAB_TITLES = new Map([
  ['rust', 'Rust'],
  ['typescript', 'Node.js'],
]);

function titleCodeGroupTabs(md) {
  md.core.ruler.push('code_group_tab_titles', (state) => {
    let insideCodeGroup = false;
    for (const token of state.tokens) {
      if (token.type === 'container_code-group_open') insideCodeGroup = true;
      if (token.type === 'container_code-group_close') insideCodeGroup = false;
      const title = TAB_TITLES.get(token.info);
      if (insideCodeGroup && token.type === 'fence' && title) token.info = `${token.info} [${title}]`;
    }
  });
}

const sidebar = [
  {
    text: 'Start here',
    items: [
      { text: 'Introduction', link: '/' },
      { text: 'Core concepts', link: '/CONCEPTS' },
      { text: 'Choose a model', link: '/MODELS' },
      { text: 'Troubleshooting', link: '/TROUBLESHOOTING' },
    ],
  },
  {
    text: 'Guides',
    items: [
      { text: 'Token matcher', link: '/TOKEN_MATCHER' },
      { text: 'Phrase matcher', link: '/PHRASE_MATCHER' },
      { text: 'Dependency matcher', link: '/DEPENDENCY_MATCHER' },
      { text: 'Editing entities', link: '/ENTITIES' },
      { text: 'Entity ruler', link: '/ENTITY_RULER' },
      { text: 'Span groups', link: '/SPAN_GROUPS' },
      { text: 'Span ruler', link: '/SPAN_RULER' },
      { text: 'Node.js', link: '/NODE' },
    ],
  },
  {
    text: 'Models',
    items: [
      { text: 'Installing a model', link: '/MODEL_INSTALLATION' },
      { text: 'Model format', link: '/MODEL_FORMAT' },
    ],
  },
  {
    text: 'Project',
    items: [
      { text: 'Compatibility', link: '/COMPATIBILITY' },
      { text: 'Differences from spaCy', link: '/SPACY_DIFFERENCES' },
      { text: 'Validation', link: '/VALIDATION' },
      { text: 'Performance', link: '/PERFORMANCE' },
    ],
  },
  {
    text: 'Contributing',
    items: [
      { text: 'Development', link: '/DEVELOPMENT' },
      { text: 'Quality', link: '/QUALITY' },
      { text: 'Releasing', link: '/RELEASING' },
      { text: 'Repository layout', link: '/REPOSITORY_LAYOUT' },
      { text: 'Progress', link: '/PROGRESS' },
    ],
  },
];

// Every guide is reachable from the sidebar, and every sidebar entry is a guide.
function requireSidebarToListEveryPage() {
  const pages = readdirSync(DOCS_DIRECTORY)
    .filter((name) => name.endsWith('.md'))
    .map((name) => (name === 'index.md' ? '/' : `/${name.slice(0, -'.md'.length)}`));
  const links = sidebar.flatMap((group) => group.items.map((item) => item.link));
  const missing = pages.filter((page) => !links.includes(page));
  const unknown = links.filter((link) => !pages.includes(link));
  if (missing.length > 0 || unknown.length > 0) {
    throw new Error(
      `The sidebar and docs/ disagree. Pages missing from the sidebar: ${missing.join(', ') || 'none'}. ` +
        `Sidebar links with no page: ${unknown.join(', ') || 'none'}.`,
    );
  }
}

requireSidebarToListEveryPage();

export default defineConfig({
  title: 'SpaRs',
  description: 'Native Rust NLP using official spaCy pretrained models, for Rust and Node.js.',
  base: BASE,
  srcDir: DOCS_DIRECTORY,
  outDir: SITE_DIRECTORY,
  cacheDir: join(SITE_DIRECTORY, '../site-cache'),
  markdown: {
    config(md) {
      linkRepositoryFiles(md);
      titleCodeGroupTabs(md);
    },
  },
  buildEnd(siteConfig) {
    linkRepositoryFilesInMarkdown(siteConfig.outDir);
  },
  themeConfig: {
    nav: [
      { text: 'spaCy', link: SPACY_URL },
      { text: 'docs.rs', link: 'https://docs.rs/spars-nlp' },
      { text: 'npm', link: 'https://www.npmjs.com/package/@spars/node' },
    ],
    sidebar,
    socialLinks: [{ icon: 'github', link: REPOSITORY_URL }],
    search: { provider: 'local' },
  },
  vite: {
    // The home page holds the credit to spaCy, the quick setup and the first example, so it
    // belongs in llms.txt with the other pages.
    plugins: [llmstxt({ excludeIndexPage: false })],
    // Pages live in docs/, outside the folder that holds node_modules, so the `vue` imports in
    // compiled pages are resolved from this file instead.
    resolve: { alias: [{ find: /^vue(?=$|\/)/, replacement: VUE_PACKAGE_DIRECTORY }] },
  },
});
