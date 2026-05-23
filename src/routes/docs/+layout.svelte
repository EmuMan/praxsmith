<script lang="ts">
    import { onMount } from "svelte";

    let { children } = $props();

    interface Heading {
        id: string;
        text: string;
        level: number;
    }

    let article: HTMLElement;
    let headings = $state<Heading[]>([]);
    let activeId = $state<string | null>(null);

    onMount(() => {
        const nodes = Array.from(
            article.querySelectorAll<HTMLHeadingElement>("h1, h2, h3"),
        );
        headings = nodes.map((el) => ({
            id: el.id,
            text: el.textContent?.replace(/#$/, "").trim() ?? "",
            level: Number(el.tagName[1]),
        }));

        // Highlight whichever heading was most recently scrolled past. Using a
        // scroll handler instead of IntersectionObserver guarantees every
        // heading gets its turn — observers can skip headings that are clustered
        // together or that never reach the active zone near the page bottom.
        const THRESHOLD = 80; // px from top of viewport
        const update = () => {
            let current = nodes[0]?.id ?? null;
            for (const el of nodes) {
                if (el.getBoundingClientRect().top - THRESHOLD <= 0) {
                    current = el.id;
                } else {
                    break;
                }
            }
            activeId = current;
        };
        update();
        const onScroll = () => requestAnimationFrame(update);
        window.addEventListener("scroll", onScroll, { passive: true });
        window.addEventListener("resize", onScroll);
        return () => {
            window.removeEventListener("scroll", onScroll);
            window.removeEventListener("resize", onScroll);
        };
    });
</script>

<div class="docs">
    <aside class="toc">
        <a class="toc-home" href="/">← simulator</a>
        <span class="toc-label">on this page</span>
        <nav>
            <ul>
                {#each headings as h (h.id)}
                    <li class:top={h.level === 1} class:sub={h.level === 3}>
                        <a href={`#${h.id}`} class:active={activeId === h.id}>
                            {h.text}
                        </a>
                    </li>
                {/each}
            </ul>
        </nav>
    </aside>

    <main class="docs-content" bind:this={article}>
        {@render children()}
    </main>
</div>

<style>
    .docs {
        display: grid;
        grid-template-columns: 16rem minmax(0, 48rem);
        gap: 3rem;
        justify-content: center;
        align-items: start;
        max-width: 72rem;
        margin: 0 auto;
        padding: 2.5rem 1.5rem 6rem;
        color: #2a2622;
    }

    /* Sticky sidebar table of contents. */
    .toc {
        position: sticky;
        top: 2.5rem;
        display: flex;
        flex-direction: column;
        gap: 0.6rem;
        max-height: calc(100vh - 5rem);
        overflow-y: auto;
        font-size: 0.85rem;
    }

    .toc-home {
        color: #7b7264;
        text-decoration: none;
        font-size: 0.8rem;
    }
    .toc-home:hover {
        color: #2a2622;
    }

    .toc-label {
        font-size: 0.72rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
        border-bottom: 1px dotted #c9bfae;
        padding-bottom: 0.4rem;
    }

    .toc nav ul {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0.15rem;
    }

    .toc li.sub {
        padding-left: 1.8rem;
    }
    .toc li.top {
        font-weight: 600;
        margin-top: 0.5rem;
    }
    .toc li.top:first-child {
        margin-top: 0;
    }
    .toc li:not(.top):not(.sub) {
        padding-left: 0.9rem;
    }

    .toc a {
        display: block;
        padding: 0.2rem 0.4rem;
        color: #6b6356;
        text-decoration: none;
        border-left: 2px solid transparent;
        transition:
            color 120ms ease,
            border-color 120ms ease;
    }
    .toc a:hover {
        color: #2a2622;
    }
    .toc a.active {
        color: #9a3b2f;
        border-left-color: #9a3b2f;
    }

    /* Content column. */
    .docs-content {
        line-height: 1.7;
        font-size: 0.98rem;
    }

    .docs-content :global(h1) {
        font-size: 2rem;
        margin: 0 0 1.5rem;
    }
    .docs-content :global(h2) {
        font-size: 1.4rem;
        margin: 2.5rem 0 0.8rem;
        padding-bottom: 0.3rem;
        border-bottom: 1px dotted #c9bfae;
    }
    .docs-content :global(h3) {
        font-size: 1.1rem;
        margin: 1.8rem 0 0.6rem;
    }

    /* Headings are scroll anchors; offset so they aren't flush to the top. */
    .docs-content :global(h1),
    .docs-content :global(h2),
    .docs-content :global(h3) {
        scroll-margin-top: 1.5rem;
    }

    /* The autolink "#" anchor appended to each heading. */
    .docs-content :global(.heading-anchor) {
        margin-left: 0.4rem;
        color: #c9bfae;
        text-decoration: none;
        opacity: 0;
        transition: opacity 120ms ease;
    }
    .docs-content :global(h1:hover .heading-anchor),
    .docs-content :global(h2:hover .heading-anchor),
    .docs-content :global(h3:hover .heading-anchor) {
        opacity: 1;
    }

    .docs-content :global(a) {
        color: #9a3b2f;
        text-decoration: underline;
        text-underline-offset: 2px;
    }

    /* Code blocks (from the mdsvex highlighter) and inline code. */
    .docs-content :global(.code-block) {
        background: #fbf7ef;
        border: 1px solid #c9bfae;
        border-radius: 2px;
        padding: 0.9rem 1rem;
        overflow-x: auto;
        font-size: 0.85rem;
        line-height: 1.55;
    }
    .docs-content :global(.code-block code),
    .docs-content :global(:not(pre) > code) {
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
    }
    .docs-content :global(:not(pre) > code) {
        background: #f3ede1;
        border: 1px solid #e0d7c6;
        border-radius: 2px;
        padding: 0.08rem 0.3rem;
        font-size: 0.85em;
    }

    /* Callout blocks: :::note / :::warning / :::tip / :::info */
    .docs-content :global(.callout) {
        border: 1px solid;
        border-left-width: 4px;
        border-radius: 2px;
        padding: 0.75rem 1rem;
        margin: 1.2rem 0;
    }
    .docs-content :global(.callout > :first-child) {
        margin-top: 0;
    }
    .docs-content :global(.callout > :last-child) {
        margin-bottom: 0;
    }
    .docs-content :global(.callout-title) {
        font-weight: 600;
        margin-bottom: 0.3rem;
    }
    .docs-content :global(.callout-note) {
        background: #f1f4f7;
        border-color: #2f6f9a;
    }
    .docs-content :global(.callout-info) {
        background: #f1f4f7;
        border-color: #2f6f9a;
    }
    .docs-content :global(.callout-tip) {
        background: #eef6ea;
        border-color: #1f6f5c;
    }
    .docs-content :global(.callout-warning) {
        background: #fbf0ec;
        border-color: #9a3b2f;
    }

    .docs-content :global(table) {
        border-collapse: collapse;
        width: 100%;
        margin: 1.2rem 0;
        font-size: 0.9rem;
    }
    .docs-content :global(th),
    .docs-content :global(td) {
        border: 1px solid #c9bfae;
        padding: 0.4rem 0.6rem;
        text-align: left;
    }
    .docs-content :global(th) {
        background: #f3ede1;
    }

    .docs-content :global(blockquote) {
        border-left: 3px solid #c9bfae;
        margin: 1.2rem 0;
        padding-left: 1rem;
        color: #6b6356;
    }

    @media (max-width: 860px) {
        .docs {
            grid-template-columns: minmax(0, 1fr);
        }
        .toc {
            position: static;
            max-height: none;
        }
    }
</style>
