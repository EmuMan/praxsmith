<script lang="ts">
    interface Props {
        actions: string[];
        actorName: string | null;
        pending: boolean;
        onchoose: (index: number) => void;
    }

    let { actions, actorName, pending, onchoose }: Props = $props();
</script>

<div class="actions">
    <span class="actions-label">
        {#if actorName}
            {actorName}'s move
        {:else}
            pick someone first
        {/if}
    </span>
    <div class="actions-row">
        {#if actorName && actions.length === 0}
            <span class="empty">nothing they can do right now.</span>
        {/if}
        {#each actions as label, i (i)}
            <button
                class="action"
                disabled={pending}
                onclick={() => onchoose(i)}
            >
                {label}
            </button>
        {/each}
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

    .empty {
        font-style: italic;
        color: #7b7264;
        font-size: 0.9rem;
    }
</style>
