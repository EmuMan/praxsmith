<script lang="ts">
    import { tick } from "svelte";
    import type { ChatMessage } from "./types";

    interface Props {
        messages: ChatMessage[];
    }

    let { messages }: Props = $props();

    let chatEl: HTMLDivElement;

    $effect(() => {
        // re-run whenever a new message lands
        messages.length;
        tick().then(() => {
            if (chatEl) chatEl.scrollTop = chatEl.scrollHeight;
        });
    });
</script>

<div class="chat" bind:this={chatEl}>
    {#if messages.length === 0}
        <p class="scene">nothing has happened yet.</p>
    {/if}
    {#each messages as msg, i (i)}
        {#if msg.kind === "system"}
            <p class="scene">{msg.line}</p>
        {:else}
            <div class="line">
                <span class="speaker">{msg.speaker}</span>
                <span class="em">—</span>
                <span class="said">{msg.line}</span>
            </div>
        {/if}
    {/each}
</div>

<style>
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
</style>
