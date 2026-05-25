import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { mdsvex } from 'mdsvex';
import rehypeSlug from 'rehype-slug';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';
import { visit } from 'unist-util-visit';

const CALLOUTS = ['note', 'warning', 'tip', 'info'];
const capitalize = (s) => s[0].toUpperCase() + s.slice(1);

// Turn GitHub-style callout blockquotes into styled <aside> blocks:
//
//   > [!warning] Optional title
//   > Body text…
//
// Implemented as a tree transform (not a parser extension) so it works with
// mdsvex's legacy remark. A blockquote whose first line is `[!type]` becomes
// `<aside class="callout callout-type">`; the rest of that line, if any, is the
// title, otherwise the type name is used. Plain blockquotes are left alone.
function remarkCallouts() {
	return (tree) => {
		visit(tree, 'blockquote', (node) => {
			const para = node.children[0];
			if (!para || para.type !== 'paragraph') return;

			// The parser reads `[!type]` as an (undefined) link reference.
			const mark = para.children[0];
			if (mark?.type !== 'linkReference' || !mark.identifier?.startsWith('!')) return;
			const type = mark.identifier.slice(1).toLowerCase();
			if (!CALLOUTS.includes(type)) return;

			para.children.shift();

			// Split an optional inline title off the first line of the body.
			let title = capitalize(type);
			const next = para.children[0];
			if (next?.type === 'text') {
				const nl = next.value.indexOf('\n');
				const firstLine = (nl === -1 ? next.value : next.value.slice(0, nl)).trim();
				if (firstLine) title = firstLine;
				next.value = nl === -1 ? '' : next.value.slice(nl + 1);
				if (next.value === '') para.children.shift();
			}
			if (para.children.length === 0) node.children.shift();

			node.children.unshift({
				type: 'paragraph',
				data: { hName: 'p', hProperties: { className: ['callout-title'] } },
				children: [{ type: 'text', value: title }]
			});

			const data = node.data || (node.data = {});
			data.hName = 'aside';
			data.hProperties = { className: ['callout', `callout-${type}`] };
		});
	};
}

// Self-contained code highlighter: escape every char significant to HTML or to
// the Svelte compiler (notably `{` and `}`, which the DSL uses heavily). No
// external Prism dependency.
const ESCAPES = {
	'&': '&amp;',
	'<': '&lt;',
	'>': '&gt;',
	'{': '&#123;',
	'}': '&#125;',
	'`': '&#96;'
};

const mdsvexOptions = {
	extensions: ['.svx'],
	remarkPlugins: [remarkCallouts],
	rehypePlugins: [
		rehypeSlug,
		[
			rehypeAutolinkHeadings,
			{
				behavior: 'append',
				properties: { class: 'heading-anchor', ariaHidden: true, tabIndex: -1 },
				content: { type: 'text', value: '#' }
			}
		]
	],
	highlight: {
		highlighter: (code, lang) => {
			const escaped = code.replace(/[&<>{}`]/g, (c) => ESCAPES[c]);
			return `<pre class="code-block" data-lang="${lang ?? ''}"><code>${escaped}</code></pre>`;
		}
	}
};

/** @type {import('@sveltejs/kit').Config} */
const config = {
	extensions: ['.svelte', '.svx'],
	preprocess: [vitePreprocess(), mdsvex(mdsvexOptions)],
	kit: {
		adapter: adapter({
			fallback: 'index.html'
		})
	}
};

export default config;
