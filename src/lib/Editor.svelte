<script lang="ts">
    import ErrorPane from "./ErrorPane.svelte";
    import CodeEditor from "./CodeEditor.svelte";

    interface Props {
        types: string;
        world: string;
        error: string | null;
        pending: boolean;
        onbuild: () => void;
    }

    let {
        types = $bindable(),
        world = $bindable(),
        error,
        pending,
        onbuild,
    }: Props = $props();
</script>

<section class="editor">
    <header class="editor-head">
        <h2 class="section-title">construction</h2>
        <button class="build" onclick={onbuild} disabled={pending}>
            {pending ? "building…" : "build world"}
        </button>
    </header>

    <div class="panes">
        <div class="pane">
            <span class="pane-label">types</span>
            <CodeEditor bind:value={types} placeholder="type definitions…" />
        </div>
        <div class="pane">
            <span class="pane-label">world</span>
            <CodeEditor bind:value={world} placeholder="world definition…" />
        </div>
    </div>

    <ErrorPane message={error} />
</section>

<style>
    .editor {
        display: flex;
        flex-direction: column;
        gap: 0.9rem;
    }

    .editor-head {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
    }

    .section-title {
        font-size: 0.75rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
        font-weight: 500;
        margin: 0;
        border-bottom: 1px dotted #c9bfae;
        padding-bottom: 0.4rem;
        flex: 1;
    }

    .build {
        font-family: inherit;
        font-size: 0.95rem;
        background: #fbf7ef;
        border: 1px solid #8a7f6d;
        color: #2a2622;
        padding: 0.45rem 0.95rem;
        cursor: pointer;
        margin-left: 1rem;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .build:hover:not(:disabled) {
        background: #2a2622;
        color: #fbf7ef;
    }

    .build:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .panes {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }

    .pane {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        min-width: 0;
    }

    .pane-label {
        font-size: 0.72rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
    }

    @media (max-width: 720px) {
        .panes {
            grid-template-columns: 1fr;
        }
    }
</style>
