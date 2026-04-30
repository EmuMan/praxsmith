<script lang="ts">
    import { onMount, tick } from "svelte";
    import init, {
        World,
        type Character,
        type Message,
        type Action,
        type WorldState,
    } from "world";

    let world: World | null = null;
    let state: WorldState | null = null;
    let chatEl: HTMLDivElement;
    let debugOpen = false;
    let hoveredCharacter: string | null = null;
    let pending = false;

    onMount(async () => {
        await init();
        world = new World();
        state = world.getState();
        await tick();
        scrollChatToBottom();
    });

    function scrollChatToBottom() {
        if (chatEl) chatEl.scrollTop = chatEl.scrollHeight;
    }

    async function choose(action: Action) {
        if (!world || pending) return;
        pending = true;
        world.applyAction(action.id);
        state = world.getState();
        await tick();
        scrollChatToBottom();
        pending = false;
    }

    $: characters = (state?.characters ?? []) as Character[];
    $: messages = (state?.messages ?? []) as Message[];
    $: actions = (state?.actions ?? []) as Action[];
</script>

<main class="page">
    <header class="masthead">
        <h1>the check in</h1>
        <p class="subtitle">
            a small demonstration of world state, crossing the boundary
        </p>
    </header>

    <section class="layout">
        <aside class="cast">
            <h2 class="section-title">cast</h2>
            {#each characters as c (c.id)}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_no_noninteractive_tabindex -->
                <article
                    class="card"
                    tabindex="0"
                    on:mouseenter={() => (hoveredCharacter = c.id)}
                    on:mouseleave={() => (hoveredCharacter = null)}
                    on:focus={() => (hoveredCharacter = c.id)}
                    on:blur={() => (hoveredCharacter = null)}
                >
                    <div class="card-head">
                        <span class="card-name">{c.name}</span>
                        <span class="card-dot" aria-hidden="true"></span>
                    </div>
                    <p class="card-bio">{c.bio}</p>
                    {#if hoveredCharacter === c.id}
                        <span class="tag">felt: {c.emotion}</span>
                    {/if}
                </article>
            {/each}
        </aside>

        <div class="chat-column">
            <div class="chat" bind:this={chatEl}>
                {#each messages as m (m.id)}
                    {#if m.system}
                        <p class="scene">{m.text}</p>
                    {:else}
                        <div class="line">
                            <span class="speaker">{m.sender}</span>
                            <span class="em">—</span>
                            <span class="said">{m.text}</span>
                        </div>
                    {/if}
                {/each}
            </div>

            <div class="actions">
                <span class="actions-label">your move</span>
                <div class="actions-row">
                    {#each actions as a (a.id)}
                        <button
                            class="action"
                            disabled={pending}
                            on:click={() => choose(a)}
                        >
                            {a.label}
                        </button>
                    {/each}
                </div>
            </div>
        </div>
    </section>

    <div class="debug" class:open={debugOpen}>
        <button class="debug-toggle" on:click={() => (debugOpen = !debugOpen)}>
            {debugOpen ? "— debug" : "+ debug"}
        </button>
        {#if debugOpen}
            <pre>{JSON.stringify(state, null, 2)}</pre>
        {/if}
    </div>
</main>

<style>
    :global(html, body) {
        margin: 0;
        padding: 0;
        background: #f5f0e8;
        color: #2a2622;
        font-family: "Iowan Old Style", "Palatino Linotype", Georgia, serif;
    }

    :global(*) {
        box-sizing: border-box;
    }

    .page {
        max-width: 1100px;
        margin: 0 auto;
        padding: 3rem 2rem 6rem;
    }

    .masthead {
        border-bottom: 1px solid #c9bfae;
        padding-bottom: 1.25rem;
        margin-bottom: 2rem;
    }

    .masthead h1 {
        font-weight: 500;
        font-size: 2.1rem;
        margin: 0;
        letter-spacing: 0.01em;
    }

    .subtitle {
        margin: 0.35rem 0 0;
        font-style: italic;
        font-size: 0.95rem;
        color: #6a6155;
    }

    .layout {
        display: grid;
        grid-template-columns: 260px 1fr;
        gap: 2.25rem;
        align-items: start;
    }

    .section-title {
        font-size: 0.75rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
        font-weight: 500;
        margin: 0 0 0.9rem;
        border-bottom: 1px dotted #c9bfae;
        padding-bottom: 0.4rem;
    }

    .cast {
        display: flex;
        flex-direction: column;
        gap: 0.9rem;
    }

    .card {
        position: relative;
        border: 1px solid #c9bfae;
        background: #fbf7ef;
        padding: 0.85rem 0.95rem;
        outline: none;
        transition:
            border-color 120ms ease,
            background 120ms ease;
    }

    .card:hover,
    .card:focus-visible {
        border-color: #7b7264;
        background: #fffbf3;
    }

    .card-head {
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    .card-name {
        font-size: 1.05rem;
    }

    .card-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: #8a7f6d;
        display: inline-block;
    }

    .card-bio {
        margin: 0.35rem 0 0;
        font-size: 0.85rem;
        color: #5a5247;
        font-style: italic;
        line-height: 1.45;
    }

    .tag {
        position: absolute;
        top: -10px;
        right: -10px;
        background: #fffbf3;
        border: 1px solid #8a7f6d;
        padding: 0.15rem 0.5rem;
        font-size: 0.72rem;
        font-family: "Patrick Hand", "Caveat", "Comic Sans MS", cursive;
        transform: rotate(-2deg);
        box-shadow: 1px 1px 0 #c9bfae;
        white-space: nowrap;
        color: #3c362e;
    }

    .chat-column {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        min-width: 0;
    }

    .chat {
        border: 1px solid #c9bfae;
        background: #fbf7ef;
        padding: 1.5rem 1.75rem;
        min-height: 380px;
        max-height: 520px;
        overflow-y: auto;
        line-height: 1.7;
        font-size: 1.02rem;
    }

    .scene {
        font-style: italic;
        color: #6a6155;
        margin: 0 0 1rem;
        padding-left: 1rem;
        border-left: 2px solid #d8cdb8;
    }

    .line {
        margin: 0 0 0.55rem;
    }

    .speaker {
        font-variant: small-caps;
        letter-spacing: 0.04em;
        color: #3c362e;
        font-weight: 500;
    }

    .em {
        color: #a79a82;
        margin: 0 0.4rem;
    }

    .said {
        color: #2a2622;
    }

    .actions {
        border-top: 1px dotted #c9bfae;
        padding-top: 1rem;
    }

    .actions-label {
        font-size: 0.72rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
        display: block;
        margin-bottom: 0.6rem;
    }

    .actions-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.6rem;
    }

    .action {
        font-family: inherit;
        font-size: 0.95rem;
        background: #fbf7ef;
        border: 1px solid #8a7f6d;
        color: #2a2622;
        padding: 0.55rem 0.95rem;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .action:hover:not(:disabled) {
        background: #2a2622;
        color: #fbf7ef;
    }

    .action:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .debug {
        position: fixed;
        bottom: 1rem;
        right: 1rem;
        background: #1a1916;
        color: #c9bfae;
        font-family: "SF Mono", "Menlo", "Consolas", monospace;
        font-size: 0.72rem;
        border: 1px solid #3a352e;
        opacity: 0.55;
        transition: opacity 120ms ease;
        max-width: 420px;
        max-height: 55vh;
        overflow: auto;
    }

    .debug.open {
        opacity: 0.96;
    }

    .debug:hover {
        opacity: 0.96;
    }

    .debug-toggle {
        display: block;
        width: 100%;
        text-align: left;
        background: transparent;
        color: inherit;
        border: none;
        border-bottom: 1px solid #3a352e;
        padding: 0.35rem 0.6rem;
        font-family: inherit;
        font-size: 0.7rem;
        cursor: pointer;
        letter-spacing: 0.1em;
    }

    .debug pre {
        margin: 0;
        padding: 0.6rem 0.8rem;
        white-space: pre-wrap;
        word-break: break-word;
    }

    @media (max-width: 720px) {
        .layout {
            grid-template-columns: 1fr;
        }
    }
</style>
