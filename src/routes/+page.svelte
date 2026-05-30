<script lang="ts">
    import { onMount } from "svelte";
    import { browser } from "$app/environment";
    import init, { PraxsmthApi } from "praxsmth";
    import type { RelationInfo } from "praxsmth";
    import type {
        AvailableAction,
        ActorInfo,
        Dialog,
        ChatMessage,
    } from "$lib/types";
    import { DEFAULT_TYPES, DEFAULT_WORLD } from "$lib/defaults";
    import Editor from "$lib/Editor.svelte";
    import Cast from "$lib/Cast.svelte";
    import Chat from "$lib/Chat.svelte";
    import Actions from "$lib/Actions.svelte";
    import ErrorPane from "$lib/ErrorPane.svelte";
    import DebugPanel from "$lib/DebugPanel.svelte";

    type ActorMode = "manual" | "auto";

    let wasmReady = $state(false);
    let api: PraxsmthApi | null = $state(null);

    const STORAGE_TYPES = "praxsmth:types";
    const STORAGE_WORLD = "praxsmth:world";

    let typesSrc = $state(
        (browser && localStorage.getItem(STORAGE_TYPES)) || DEFAULT_TYPES,
    );
    let worldSrc = $state(
        (browser && localStorage.getItem(STORAGE_WORLD)) || DEFAULT_WORLD,
    );
    let buildError: string | null = $state(null);
    let building = $state(false);

    let actors: ActorInfo[] = $state([]);
    let relations: RelationInfo[] = $state([]);
    let emotions: Record<string, string | undefined> = $state({});
    let modes: Record<string, ActorMode> = $state({});
    let currentActorId: string | null = $state(null);
    let availableActions: AvailableAction[] = $state([]);
    let actionScoreDepth = $state(4);
    let messages: ChatMessage[] = $state([]);
    let runtimeError: string | null = $state(null);
    let pending = $state(false);
    let isTyping = $state(false);

    onMount(async () => {
        await init();
        wasmReady = true;
    });

    $effect(() => {
        localStorage.setItem(STORAGE_TYPES, typesSrc);
        localStorage.setItem(STORAGE_WORLD, worldSrc);
    });

    function reportRuntimeError(err: unknown, where: string) {
        const line = err instanceof Error ? err.message : String(err);
        runtimeError = `error in ${where}:\n${line}`;
    }

    function getActions(id: string): AvailableAction[] {
        if (!api) return [];
        try {
            return api.getAvailableActions(
                id,
                actionScoreDepth,
            ) as AvailableAction[];
        } catch (err) {
            reportRuntimeError(err, "getAvailableActions");
            return [];
        }
    }

    // Find the next actor (starting from startIdx, cycling) that has at least
    // one available action. Returns null if none do.
    function findActorWithActions(startIdx: number): string | null {
        const active = actors.filter((a) => a.is_active);
        if (active.length === 0) return null;
        const start =
            ((startIdx % active.length) + active.length) % active.length;
        for (let i = 0; i < active.length; i++) {
            const actor = active[(start + i) % active.length];
            if (getActions(actor.id).length > 0) return actor.id;
        }
        return null;
    }

    function indexOfActiveActor(id: string | null): number {
        if (!id) return 0;
        const active = actors.filter((a) => a.is_active);
        const i = active.findIndex((a) => a.id === id);
        return i === -1 ? 0 : i;
    }

    function refreshFromApi() {
        if (!api) return;
        try {
            actors = api.getActorInfo() as ActorInfo[];
            relations = api.getRelationInfo() as RelationInfo[];

            const nextEmotions: Record<string, string | undefined> = {};
            const nextModes: Record<string, ActorMode> = {};
            for (const a of actors) {
                nextEmotions[a.id] = api.getCurrentEmotion(a.id) ?? undefined;
                nextModes[a.id] = modes[a.id] ?? "manual";
            }
            emotions = nextEmotions;
            modes = nextModes;

            // If the current actor is gone or inactive, drop them — the next
            // turn advancement (or initial seeding) will pick someone new.
            // Otherwise leave the turn alone; advancement is explicit so
            // onUpdate firing mid-action doesn't double-advance.
            if (currentActorId) {
                const still = actors.find((a) => a.id === currentActorId);
                if (!still || !still.is_active) currentActorId = null;
            }

            availableActions = currentActorId ? getActions(currentActorId) : [];
        } catch (err) {
            reportRuntimeError(err, "refreshFromWorld");
        }
    }

    function isHidden(id: string | null): boolean {
        if (!id) return false;
        return actors.find((a) => a.id === id)?.is_hidden ?? false;
    }

    // Point the turn at the first actor with actions from `startIdx`, loading
    // their actions. Inactive actors are never candidates (findActorWithActions
    // only considers is_active), so their turns are skipped entirely.
    function selectNextActor(startIdx: number) {
        currentActorId = findActorWithActions(startIdx);
        availableActions = currentActorId ? getActions(currentActorId) : [];
    }

    function seedTurn() {
        selectNextActor(indexOfActiveActor(currentActorId));
        autoRunHidden();
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
            currentActorId = null;
            modes = {};
            messages = [];
            refreshFromApi();
            seedTurn();
        } catch (err) {
            buildError = err instanceof Error ? err.message : String(err);
            api = null;
        } finally {
            building = false;
        }
    }

    function toggleMode(id: string) {
        modes = {
            ...modes,
            [id]: modes[id] === "auto" ? "manual" : "auto",
        };
    }

    function advanceTurn() {
        // Move past the current actor before searching, so the same actor
        // doesn't immediately re-run when they still have actions.
        selectNextActor(indexOfActiveActor(currentActorId) + 1);
        autoRunHidden();
    }

    // Hidden actors are picked for turns like any other active actor, but the
    // player never drives them: whenever the turn lands on a hidden actor, we
    // auto-pick and apply their action, then advance — looping until a visible
    // (non-hidden) actor is up, or no one can act. Deliberately separate from
    // inactive-skipping (handled in turn selection) and from the per-actor
    // "auto" mode (which still waits for a manual "next"). Note we cannot gate
    // this on `pending`, since it runs synchronously inside chooseAction's
    // pending window.
    function autoRunHidden() {
        if (!api) return;
        let guard = 0;
        while (isHidden(currentActorId)) {
            if (guard++ > 10_000) {
                reportRuntimeError(
                    new Error("hidden actor turn limit exceeded"),
                    "autoRunHidden",
                );
                break;
            }
            // Turn selection only lands on actors with actions, but guard in
            // case a hidden actor's options vanished out from under us.
            if (availableActions.length === 0) break;
            const actorId = currentActorId!;
            const idx = pickAutoChoice(availableActions);
            try {
                api.applyAction(actorId, idx);
            } catch (err) {
                reportRuntimeError(err, "applyAction");
                break;
            }
            // applyAction fires onUpdate -> refreshFromApi, which may have
            // already cleared currentActorId if this actor deactivated itself;
            // advancing past the actor that just acted is correct either way.
            selectNextActor(indexOfActiveActor(actorId) + 1);
        }
    }

    function chooseAction(index: number) {
        if (!api || !currentActorId || pending || isTyping) return;
        const actorId = currentActorId;
        pending = true;
        try {
            api.applyAction(actorId, index);
            advanceTurn();
        } catch (err) {
            reportRuntimeError(err, "applyAction");
        } finally {
            pending = false;
        }
    }

    function pickAutoChoice(actions: AvailableAction[]): number {
        let best = -Infinity;
        for (const a of actions) if (a.score > best) best = a.score;
        const tied: number[] = [];
        for (let i = 0; i < actions.length; i++) {
            if (actions[i].score === best) tied.push(i);
        }
        return tied[Math.floor(Math.random() * tied.length)];
    }

    function nextAuto() {
        if (!api || !currentActorId || pending || isTyping) return;
        if (availableActions.length === 0) return;
        const idx = pickAutoChoice(availableActions);
        chooseAction(idx);
    }

    function reset() {
        api = null;
        actors = [];
        emotions = {};
        modes = {};
        currentActorId = null;
        availableActions = [];
        messages = [];
        buildError = null;
        runtimeError = null;
    }

    let currentActor = $derived(
        actors.find((a) => a.id === currentActorId) ?? null,
    );
    let currentActorName = $derived(currentActor?.name ?? null);
    let currentActorMode: ActorMode | null = $derived(
        currentActorId ? (modes[currentActorId] ?? "manual") : null,
    );
    // The cast shows everyone the player is meant to see: hidden actors are
    // omitted entirely (they act behind the scenes), while inactive actors
    // still appear — they just don't take turns.
    let visibleActors = $derived(actors.filter((a) => !a.is_hidden));
    let noOneCanAct = $derived(
        visibleActors.length > 0 && currentActorId === null,
    );
</script>

<main class="page">
    <header class="masthead">
        <h1>praxsmith simulator</h1>
        <p class="subtitle">a web implementation of the praxsmith framework</p>
        <a class="docs-link" href="/docs" target="_blank" rel="noopener">
            docs ↗
        </a>
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
                actors={visibleActors}
                currentId={currentActorId}
                {modes}
                {emotions}
                ontogglemode={toggleMode}
            />

            <div class="chat-column">
                <Chat {messages} bind:isTyping />
                <Actions
                    actions={availableActions}
                    actorName={currentActorName}
                    actorMode={currentActorMode}
                    pending={pending || isTyping}
                    {noOneCanAct}
                    onchoose={chooseAction}
                    onnext={nextAuto}
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

        <DebugPanel {api} {relations} defaultActorName={currentActorName} />
    {/if}
</main>

<style>
    .page {
        max-width: 1100px;
        margin: 0 auto;
        padding: 3rem 2rem 6rem;
    }

    .masthead {
        position: relative;
        border-bottom: 1px solid #c9bfae;
        padding-bottom: 1.25rem;
        margin-bottom: 2rem;
    }

    .docs-link {
        position: absolute;
        top: 0;
        right: 0;
        font-size: 0.8rem;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        color: #7b7264;
        text-decoration: none;
    }
    .docs-link:hover {
        color: #2a2622;
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
