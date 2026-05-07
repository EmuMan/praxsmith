<script lang="ts">
    import { onMount } from "svelte";
    import init, { World } from "praxsmth";
    import type { AgentInfo, Dialog, ChatMessage } from "$lib/types";
    import { DEFAULT_TYPES, DEFAULT_WORLD } from "$lib/defaults";
    import Editor from "$lib/Editor.svelte";
    import Cast from "$lib/Cast.svelte";
    import Chat from "$lib/Chat.svelte";
    import Actions from "$lib/Actions.svelte";

    let wasmReady = $state(false);
    let world: World | null = $state(null);

    let typesSrc = $state(DEFAULT_TYPES);
    let worldSrc = $state(DEFAULT_WORLD);
    let buildError: string | null = $state(null);
    let building = $state(false);

    let agents: AgentInfo[] = $state([]);
    let emotions: Record<string, string | undefined> = $state({});
    let selectedId: string | null = $state(null);
    let availableActions: string[] = $state([]);
    let messages: ChatMessage[] = $state([]);
    let pending = $state(false);

    onMount(async () => {
        await init();
        wasmReady = true;
    });

    function refreshFromWorld() {
        if (!world) return;
        agents = world.getAgentNames() as AgentInfo[];

        const nextEmotions: Record<string, string | undefined> = {};
        for (const a of agents) {
            nextEmotions[a.id] = world.getCurrentEmotion(a.id) ?? undefined;
        }
        emotions = nextEmotions;

        if (selectedId && !agents.some((a) => a.id === selectedId)) {
            selectedId = null;
        }

        availableActions = selectedId
            ? world.getAvailableActionNames(selectedId)
            : [];
    }

    function handleDialog(dialog: Dialog) {
        if (dialog.speaker) {
            messages = [
                ...messages,
                {
                    kind: "speech",
                    speaker: dialog.speaker,
                    line: dialog.line,
                },
            ];
        } else {
            messages = [...messages, { kind: "system", line: dialog.line }];
        }
    }

    function build() {
        if (!wasmReady || building) return;
        building = true;
        buildError = null;
        try {
            const w = World.new(typesSrc, worldSrc);
            w.setOnUpdate(() => refreshFromWorld());
            w.setOnDialog((d: Dialog) => handleDialog(d));
            world = w;
            selectedId = null;
            messages = [];
            refreshFromWorld();
        } catch (err) {
            buildError = err instanceof Error ? err.message : String(err);
            world = null;
        } finally {
            building = false;
        }
    }

    function selectAgent(id: string) {
        selectedId = id;
        if (world) {
            availableActions = world.getAvailableActionNames(id);
        }
    }

    async function chooseAction(index: number) {
        if (!world || !selectedId || pending) return;
        pending = true;
        try {
            world.applyAction(selectedId, index);
        } catch (err) {
            const line = err instanceof Error ? err.message : String(err);
            messages = [...messages, { kind: "system", line: `error: ${line}` }];
        } finally {
            pending = false;
        }
    }

    function reset() {
        world = null;
        agents = [];
        emotions = {};
        selectedId = null;
        availableActions = [];
        messages = [];
        buildError = null;
    }

    let selectedAgentName = $derived(
        agents.find((a) => a.id === selectedId)?.name ?? null,
    );
</script>

<main class="page">
    <header class="masthead">
        <h1>the check in</h1>
        <p class="subtitle">
            a small demonstration of world state, crossing the boundary
        </p>
    </header>

    {#if !world}
        <Editor
            bind:types={typesSrc}
            bind:world={worldSrc}
            error={buildError}
            pending={building || !wasmReady}
            onbuild={build}
        />
    {:else}
        <section class="layout">
            <Cast
                {agents}
                {selectedId}
                {emotions}
                onselect={selectAgent}
            />

            <div class="chat-column">
                <Chat {messages} />
                <Actions
                    actions={availableActions}
                    actorName={selectedAgentName}
                    {pending}
                    onchoose={chooseAction}
                />
            </div>
        </section>

        <div class="reset-row">
            <button class="reset" onclick={reset}>edit world</button>
        </div>
    {/if}
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

    .chat-column {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        min-width: 0;
    }

    .reset-row {
        margin-top: 2rem;
        display: flex;
        justify-content: flex-end;
    }

    .reset {
        font-family: inherit;
        font-size: 0.85rem;
        background: transparent;
        border: 1px solid #c9bfae;
        color: #6a6155;
        padding: 0.4rem 0.85rem;
        cursor: pointer;
        letter-spacing: 0.04em;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .reset:hover {
        background: #2a2622;
        color: #fbf7ef;
        border-color: #2a2622;
    }

    @media (max-width: 720px) {
        .layout {
            grid-template-columns: 1fr;
        }
    }
</style>
