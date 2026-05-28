<script lang="ts">
    import { tick } from "svelte";
    import type { ChatMessage } from "./types";

    interface Props {
        messages: ChatMessage[];
        isTyping?: boolean;
        charDelayMs?: number;
    }

    let {
        messages,
        isTyping = $bindable(false),
        charDelayMs = 18,
    }: Props = $props();

    let chatEl: HTMLDivElement;

    let typedIdx = $state(0);
    let typedChars = $state(0);

    // If messages were reset (e.g. world rebuild), pull our cursors back.
    $effect(() => {
        if (typedIdx > messages.length) {
            typedIdx = 0;
            typedChars = 0;
        }
    });

    let activeMsg = $derived(messages[typedIdx]);

    $effect(() => {
        if (!activeMsg) {
            isTyping = false;
            return;
        }
        isTyping = true;
        typedChars = 0;
        const line = activeMsg.line;
        const id = window.setInterval(() => {
            typedChars++;
            if (typedChars >= line.length) {
                window.clearInterval(id);
                typedIdx++;
            }
        }, charDelayMs);
        return () => window.clearInterval(id);
    });

    $effect(() => {
        messages.length;
        typedChars;
        typedIdx;
        tick().then(() => {
            if (chatEl) chatEl.scrollTop = chatEl.scrollHeight;
        });
    });

    function renderMsg(msg: ChatMessage, text: string) {
        return { msg, text };
    }
</script>

<div class="chat" bind:this={chatEl}>
    {#if messages.length === 0}
        <p class="scene">nothing has happened yet.</p>
    {/if}
    {#each messages.slice(0, typedIdx) as msg, i (i)}
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
    {#if activeMsg}
        {#if activeMsg.kind === "system"}
            <p class="scene">
                {activeMsg.line.slice(0, typedChars)}<span class="caret"
                ></span>
            </p>
        {:else}
            <div class="line">
                <span class="speaker">{activeMsg.speaker}</span>
                <span class="em">—</span>
                <span class="said"
                    >{activeMsg.line.slice(0, typedChars)}<span class="caret"
                    ></span></span
                >
            </div>
        {/if}
    {/if}
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

    .caret {
        display: inline-block;
        width: 0.5ch;
        background: #2a2622;
        height: 1em;
        vertical-align: -0.15em;
        margin-left: 1px;
        animation: blink 0.9s steps(2, start) infinite;
    }

    @keyframes blink {
        to {
            visibility: hidden;
        }
    }
</style>
