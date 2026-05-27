<script lang="ts">
    import type { ActorInfo } from "./types";

    interface Props {
        actors: ActorInfo[];
        selectedId: string | null;
        emotions: Record<string, string | undefined>;
        onselect: (id: string) => void;
    }

    let { actors, selectedId, emotions, onselect }: Props = $props();
</script>

<aside class="cast">
    <h2 class="section-title">cast</h2>
    {#each actors as actor (actor.id)}
        {@const emotion = emotions[actor.id]}
        <button
            class="card"
            class:selected={selectedId === actor.id}
            onclick={() => onselect(actor.id)}
        >
            <div class="card-head">
                <span class="card-name">{actor.name}</span>
                <span class="card-dot" aria-hidden="true"></span>
            </div>
            {#if emotion}
                <span class="tag">felt: {emotion}</span>
            {/if}
        </button>
    {/each}
    {#if actors.length === 0}
        <p class="empty">no one is here yet.</p>
    {/if}
</aside>

<style>
    .cast {
        display: flex;
        flex-direction: column;
        gap: 0.9rem;
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

    .card {
        position: relative;
        border: 1px solid #c9bfae;
        background: #fbf7ef;
        padding: 0.85rem 0.95rem;
        outline: none;
        cursor: pointer;
        text-align: left;
        font: inherit;
        color: inherit;
        transition:
            border-color 120ms ease,
            background 120ms ease;
    }

    .card:hover,
    .card:focus-visible {
        border-color: #7b7264;
        background: #fffbf3;
    }

    .card.selected {
        border-color: #2a2622;
        background: #fffbf3;
        box-shadow: inset 0 0 0 1px #2a2622;
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

    .empty {
        font-style: italic;
        color: #7b7264;
        font-size: 0.9rem;
        margin: 0;
    }
</style>
