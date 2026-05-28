<script lang="ts">
    import type {
        PraxsmthApi,
        PraxsmthConstant,
        RelationInfo,
    } from "praxsmth";

    interface Props {
        api: PraxsmthApi;
        relations: RelationInfo[];
        defaultActorName?: string | null;
    }

    let {
        api,
        relations,
        defaultActorName = null,
    }: Props = $props();

    let open = $state(false);

    let exprInput = $state("");
    let exprResult: string | null = $state(null);
    let exprError: string | null = $state(null);

    let effectActor = $state("");
    let effectInput = $state("");
    let effectResult: string | null = $state(null);
    let effectError: string | null = $state(null);

    function formatConstant(c: PraxsmthConstant): string {
        if ("Number" in c) return `Number(${c.Number})`;
        if ("Boolean" in c) return `Boolean(${c.Boolean})`;
        if ("Variant" in c) return `Variant(${c.Variant})`;
        if ("String" in c) return `String(${JSON.stringify(c.String)})`;
        if ("ActorRef" in c) return `ActorRef(${c.ActorRef})`;
        return JSON.stringify(c);
    }

    function errMessage(err: unknown): string {
        return err instanceof Error ? err.message : String(err);
    }

    function runEvaluate() {
        exprResult = null;
        exprError = null;
        try {
            const value = api.evaluateExpression(exprInput);
            exprResult = formatConstant(value);
        } catch (err) {
            exprError = errMessage(err);
        }
    }

    let hoveredRelation: RelationInfo | null = $state(null);
    let tooltipBottom = $state(0);
    let tooltipRight = $state(0);

    function showRelationTooltip(relation: RelationInfo, e: MouseEvent) {
        const target = e.currentTarget as HTMLElement;
        const rect = target.getBoundingClientRect();
        tooltipBottom = window.innerHeight - rect.bottom;
        tooltipRight = window.innerWidth - rect.left + 8;
        hoveredRelation = relation;
    }

    function hideRelationTooltip() {
        hoveredRelation = null;
    }

    function runProcessEffect() {
        effectResult = null;
        effectError = null;
        const actor = effectActor.trim() || defaultActorName?.trim() || "";
        if (!actor) {
            effectError = "actor name is required";
            return;
        }
        try {
            const dialog = api.processEffect(actor, effectInput);
            if (dialog) {
                effectResult = dialog.speaker
                    ? `${dialog.speaker}: ${dialog.line}`
                    : dialog.line;
            } else {
                effectResult = "(no dialog produced)";
            }
        } catch (err) {
            effectError = errMessage(err);
        }
    }
</script>

<div class="debug-root" class:open>
    <button class="toggle" onclick={() => (open = !open)} aria-expanded={open}>
        {open ? "× close debug" : "debug"}
    </button>

    {#if open}
        <div class="panel" role="region" aria-label="debug panel">
            <section class="block">
                <h3>evaluate expression</h3>
                <textarea
                    class="input"
                    rows="2"
                    placeholder="e.g. 'a.trusts.b.level'"
                    bind:value={exprInput}
                ></textarea>
                <div class="row">
                    <button class="run" onclick={runEvaluate}>evaluate</button>
                </div>
                {#if exprResult !== null}
                    <pre class="result">{exprResult}</pre>
                {/if}
                {#if exprError}
                    <pre class="error">{exprError}</pre>
                {/if}
            </section>

            <section class="block">
                <h3>process effect</h3>
                <input
                    class="input"
                    type="text"
                    placeholder={defaultActorName
                        ? `actor name (default: ${defaultActorName})`
                        : "actor name"}
                    bind:value={effectActor}
                />
                <textarea
                    class="input"
                    rows="3"
                    placeholder="effect, e.g. 'activate @actor'"
                    bind:value={effectInput}
                ></textarea>
                <div class="row">
                    <button class="run" onclick={runProcessEffect}>
                        process
                    </button>
                </div>
                {#if effectResult !== null}
                    <pre class="result">{effectResult}</pre>
                {/if}
                {#if effectError}
                    <pre class="error">{effectError}</pre>
                {/if}
            </section>

            <section class="block">
                <h3>relations ({relations.length})</h3>
                {#if relations.length === 0}
                    <p class="empty">no relations</p>
                {:else}
                    <ul class="relations">
                        {#each relations as relation, i (i)}
                            <li
                                class="relation"
                                onmouseenter={(e) =>
                                    showRelationTooltip(relation, e)}
                                onmouseleave={hideRelationTooltip}
                            >
                                <span class="relation-sentence"
                                    >{relation.sentence}</span
                                >
                            </li>
                        {/each}
                    </ul>
                {/if}
            </section>
        </div>
    {/if}

    {#if hoveredRelation}
        <div
            class="relation-tooltip"
            role="tooltip"
            style="bottom: {tooltipBottom}px; right: {tooltipRight}px;"
        >
            <div class="tt-row">
                <span class="tt-label">type</span>
                <span class="tt-value">{hoveredRelation.type_id}</span>
            </div>
            <div class="tt-row">
                <span class="tt-label">actors</span>
                <span class="tt-value"
                    >{hoveredRelation.actors.join(", ")}</span
                >
            </div>
            {#if hoveredRelation.fields.length > 0}
                <div class="tt-row">
                    <span class="tt-label">fields</span>
                    <span class="tt-value">
                        {#each hoveredRelation.fields as [k, v]}
                            <div>{k} = {formatConstant(v)}</div>
                        {/each}
                    </span>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .debug-root {
        position: fixed;
        bottom: 1.25rem;
        right: 1.25rem;
        z-index: 50;
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0.6rem;
    }

    .toggle {
        font-family: inherit;
        font-size: 0.78rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        background: #2a2622;
        color: #fbf7ef;
        border: 1px solid #2a2622;
        padding: 0.5rem 0.9rem;
        cursor: pointer;
        box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
    }

    .toggle:hover {
        background: #45403a;
        border-color: #45403a;
    }

    .panel {
        width: min(380px, calc(100vw - 2.5rem));
        max-height: 70vh;
        overflow-y: auto;
        background: #fbf7ef;
        border: 1px solid #8a7f6d;
        padding: 1rem 1.1rem;
        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
        display: flex;
        flex-direction: column;
        gap: 1.1rem;
    }

    .block {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .block + .block {
        border-top: 1px dotted #c9bfae;
        padding-top: 1rem;
    }

    h3 {
        margin: 0;
        font-weight: 500;
        font-size: 0.72rem;
        letter-spacing: 0.18em;
        text-transform: uppercase;
        color: #7b7264;
    }

    .input {
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.85rem;
        background: #fff;
        border: 1px solid #c9bfae;
        color: #2a2622;
        padding: 0.45rem 0.55rem;
        width: 100%;
        box-sizing: border-box;
        resize: vertical;
    }

    .input:focus {
        outline: none;
        border-color: #2a2622;
    }

    .row {
        display: flex;
        justify-content: flex-end;
    }

    .run {
        font-family: inherit;
        font-size: 0.85rem;
        background: #fbf7ef;
        border: 1px solid #8a7f6d;
        color: #2a2622;
        padding: 0.4rem 0.85rem;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease;
    }

    .run:hover {
        background: #2a2622;
        color: #fbf7ef;
    }

    .result {
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.8rem;
        background: #f1ecdf;
        border: 1px solid #c9bfae;
        color: #2a2622;
        padding: 0.5rem 0.65rem;
        margin: 0;
        white-space: pre-wrap;
        overflow-x: auto;
    }

    .empty {
        margin: 0;
        font-size: 0.8rem;
        color: #7b7264;
        font-style: italic;
    }

    .relations {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0.3rem;
    }

    .relation {
        position: relative;
        font-size: 0.82rem;
        color: #2a2622;
        padding: 0.35rem 0.5rem;
        background: #f1ecdf;
        border: 1px solid #c9bfae;
        cursor: default;
    }

    .relation-sentence {
        display: block;
    }

    .relation-tooltip {
        position: fixed;
        width: 240px;
        background: #2a2622;
        color: #fbf7ef;
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.72rem;
        padding: 0.5rem 0.6rem;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
        z-index: 60;
        pointer-events: none;
        white-space: normal;
        word-break: break-word;
    }

    .tt-row {
        display: flex;
        flex-direction: column;
        gap: 0.1rem;
    }

    .tt-row + .tt-row {
        margin-top: 0.4rem;
    }

    .tt-label {
        font-size: 0.62rem;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: #c9bfae;
    }

    .tt-value {
        color: #fbf7ef;
    }

    .error {
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.8rem;
        background: #fbecec;
        border: 1px solid #c9a0a0;
        color: #6a2222;
        padding: 0.5rem 0.65rem;
        margin: 0;
        white-space: pre-wrap;
        overflow-x: auto;
    }
</style>
