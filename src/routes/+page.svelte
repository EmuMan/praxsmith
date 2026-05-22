<script lang="ts">
    import { onMount } from "svelte";
    import { browser } from "$app/environment";
    import init, { PraxsmthApi } from "praxsmth";
    import type {
        AvailableAction,
        AgentInfo,
        Dialog,
        ChatMessage,
    } from "$lib/types";
    import { DEFAULT_TYPES, DEFAULT_WORLD } from "$lib/defaults";
    import Editor from "$lib/Editor.svelte";
    import Cast from "$lib/Cast.svelte";
    import Chat from "$lib/Chat.svelte";
    import Actions from "$lib/Actions.svelte";
    import ErrorPane from "$lib/ErrorPane.svelte";

    let wasmReady = $state(false);
    let api: PraxsmthApi | null = $state(null);

    const STORAGE_TYPES = "praxsmth:types";
    const STORAGE_WORLD = "praxsmth:world";

    // Restore saved work at init (browser only) so it's in place before any
    // effect runs — avoids a default-vs-saved write race.
    let typesSrc = $state(
        (browser && localStorage.getItem(STORAGE_TYPES)) || DEFAULT_TYPES,
    );
    let worldSrc = $state(
        (browser && localStorage.getItem(STORAGE_WORLD)) || DEFAULT_WORLD,
    );
    let buildError: string | null = $state(null);
    let building = $state(false);

    let agents: AgentInfo[] = $state([]);
    let emotions: Record<string, string | undefined> = $state({});
    let selectedId: string | null = $state(null);
    let availableActions: AvailableAction[] = $state([]);
    let actionScoreDepth = $state(3);
    let messages: ChatMessage[] = $state([]);
    let runtimeError: string | null = $state(null);
    let pending = $state(false);

    onMount(async () => {
        await init();
        wasmReady = true;
    });

    // Persist edits so a refresh doesn't lose work. Effects run only in the
    // browser, so localStorage is always available here.
    $effect(() => {
        localStorage.setItem(STORAGE_TYPES, typesSrc);
        localStorage.setItem(STORAGE_WORLD, worldSrc);
    });

    function reportRuntimeError(err: unknown, where: string) {
        const line = err instanceof Error ? err.message : String(err);
        runtimeError = `error in ${where}:\n${line}`;
    }

    function refreshFromApi() {
        if (!api) return;
        try {
            agents = api.getAgentInfo() as AgentInfo[];

            const nextEmotions: Record<string, string | undefined> = {};
            for (const a of agents) {
                nextEmotions[a.id] = api.getCurrentEmotion(a.id) ?? undefined;
            }
            emotions = nextEmotions;

            if (selectedId && !agents.some((a) => a.id === selectedId)) {
                selectedId = null;
            }

            availableActions = selectedId
                ? api.getAvailableActions(selectedId, actionScoreDepth)
                : [];
        } catch (err) {
            reportRuntimeError(err, "refreshFromWorld");
        }
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
            const newApi = new PraxsmthApi(typesSrc, worldSrc);
            newApi.setOnUpdate(() => refreshFromApi());
            newApi.setOnDialog((d: Dialog) => handleDialog(d));
            api = newApi;
            selectedId = null;
            messages = [];
            refreshFromApi();
        } catch (err) {
            buildError = err instanceof Error ? err.message : String(err);
            api = null;
        } finally {
            building = false;
        }
    }

    function selectAgent(id: string) {
        selectedId = id;
        if (api) {
            try {
                availableActions = api.getAvailableActions(
                    id,
                    actionScoreDepth,
                );
            } catch (err) {
                reportRuntimeError(err, "selectAgent");
                availableActions = [];
            }
        }
    }

    async function chooseAction(index: number) {
        if (!api || !selectedId || pending) return;
        pending = true;
        try {
            api.applyAction(selectedId, index);
        } catch (err) {
            reportRuntimeError(err, "applyAction");
        } finally {
            pending = false;
        }
    }

    function reset() {
        api = null;
        agents = [];
        emotions = {};
        selectedId = null;
        availableActions = [];
        messages = [];
        buildError = null;
        runtimeError = null;
    }

    let selectedAgentName = $derived(
        agents.find((a) => a.id === selectedId)?.name ?? null,
    );

    let visibleAgents = $derived(agents.filter((a) => a.active));
</script>

<main class="page">
    <header class="masthead">
        <h1>the check in</h1>
        <p class="subtitle">
            a small demonstration of world state, crossing the boundary
        </p>
    </header>

    {#if !api}
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
                agents={visibleAgents}
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

        <div class="runtime-error-slot">
            <ErrorPane
                message={runtimeError}
                ondismiss={() => (runtimeError = null)}
            />
        </div>

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

    .runtime-error-slot:not(:empty) {
        margin-top: 1.5rem;
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
