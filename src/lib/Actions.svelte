<script lang="ts">
    import type { AvailableAction } from "$lib/types";

    type Mode = "manual" | "auto";

    interface Props {
        actions: AvailableAction[];
        actorName: string | null;
        actorMode: Mode | null;
        pending: boolean;
        noOneCanAct: boolean;
        onchoose: (index: number) => void;
        onnext: () => void;
    }

    let {
        actions,
        actorName,
        actorMode,
        pending,
        noOneCanAct,
        onchoose,
        onnext,
    }: Props = $props();
</script>

<div class="actions">
    <span class="actions-label">
        {#if noOneCanAct}
            no one can act
        {:else if actorName}
            {actorName}'s move
        {:else}
            …
        {/if}
    </span>

    <div class="actions-row">
        {#if noOneCanAct}
            <span class="empty">
                no actor has any available actions right now.
            </span>
        {:else if actorMode === "auto"}
            <button
                class="next"
                disabled={pending || actions.length === 0}
                onclick={onnext}
            >
                next →
            </button>
        {:else if actorName && actions.length === 0}
            <span class="empty">nothing they can do right now.</span>
        {:else}
            {#each actions as action, i (i)}
                <button
                    class="action"
                    disabled={pending}
                    onclick={() => onchoose(i)}
                >
                    {action.display_name} ({action.score})
                </button>
            {/each}
        {/if}
    </div>
</div>

<style>
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
        align-items: center;
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

    .next {
        font-family: inherit;
        font-size: 1.15rem;
        letter-spacing: 0.08em;
        background: #2a2622;
        color: #fbf7ef;
        border: 1px solid #2a2622;
        padding: 0.85rem 2.4rem;
        cursor: pointer;
        flex: 1;
        text-transform: uppercase;
        transition:
            background 120ms ease,
            transform 120ms ease;
    }

    .next:hover:not(:disabled) {
        background: #3c362e;
    }

    .next:active:not(:disabled) {
        transform: translateY(1px);
    }

    .next:disabled {
        opacity: 0.5;
        cursor: default;
    }

    .empty {
        font-style: italic;
        color: #7b7264;
        font-size: 0.9rem;
    }
</style>
