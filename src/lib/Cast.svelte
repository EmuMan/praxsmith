<script lang="ts">
    import type { ActorInfo } from "./types";

    type ActorMode = "manual" | "auto";

    interface Props {
        actors: ActorInfo[];
        currentId: string | null;
        modes: Record<string, ActorMode>;
        emotions: Record<string, string | undefined>;
        ontogglemode: (id: string) => void;
    }

    let { actors, currentId, modes, emotions, ontogglemode }: Props = $props();
</script>

<aside class="cast">
    <h2 class="section-title">cast</h2>
    {#each actors as actor (actor.id)}
        {@const emotion = emotions[actor.id]}
        {@const mode = modes[actor.id] ?? "manual"}
        <div
            class="card"
            class:current={currentId === actor.id}
            class:inactive={!actor.is_active}
        >
            <div class="card-head">
                <span class="card-name">{actor.name}</span>
                <span class="card-dot" aria-hidden="true"></span>
            </div>
            {#if actor.is_active}
                <button
                    class="mode"
                    class:auto={mode === "auto"}
                    onclick={() => ontogglemode(actor.id)}
                    title="toggle manual/automatic"
                >
                    {mode}
                </button>
            {:else}
                <span class="mode mode-static">inactive</span>
            {/if}
            {#if emotion}
                <span class="tag">felt: {emotion}</span>
            {/if}
        </div>
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
        text-align: left;
        font: inherit;
        color: inherit;
        transition:
            border-color 200ms ease,
            background 200ms ease,
            box-shadow 200ms ease;
    }

    .card.current {
        border-color: #2a2622;
        background: #fffbf3;
        box-shadow: inset 0 0 0 1px #2a2622;
    }

    .card.inactive {
        opacity: 0.55;
        border-style: dashed;
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

    .card.current .card-dot {
        background: #2a2622;
        box-shadow: 0 0 0 3px #d8cdb8;
    }

    .mode {
        margin-top: 0.55rem;
        font-family: inherit;
        font-size: 0.7rem;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        background: transparent;
        border: 1px solid #c9bfae;
        color: #6a6155;
        padding: 0.2rem 0.55rem;
        cursor: pointer;
        transition:
            background 120ms ease,
            color 120ms ease,
            border-color 120ms ease;
    }

    .mode:hover {
        border-color: #2a2622;
        color: #2a2622;
    }

    .mode.auto {
        background: #2a2622;
        color: #fbf7ef;
        border-color: #2a2622;
    }

    .mode-static {
        display: inline-block;
        cursor: default;
        font-style: italic;
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
