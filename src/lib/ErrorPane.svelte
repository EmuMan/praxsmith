<script lang="ts">
    interface Props {
        message: string | null;
        ondismiss?: () => void;
    }

    let { message, ondismiss }: Props = $props();
</script>

{#if message}
    <div class="pane">
        <pre class="message">{message}</pre>
        {#if ondismiss}
            <button
                class="dismiss"
                onclick={ondismiss}
                aria-label="dismiss error"
            >
                dismiss
            </button>
        {/if}
    </div>
{/if}

<style>
    .pane {
        display: flex;
        align-items: flex-start;
        gap: 0.75rem;
        background: #fbecec;
        border: 1px solid #c9a0a0;
        padding: 0.6rem 0.8rem;
    }

    .message {
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.8rem;
        color: #6a2222;
        margin: 0;
        white-space: pre-wrap;
        flex: 1;
        min-width: 0;
        overflow-x: auto;
    }

    .dismiss {
        font-family: inherit;
        font-size: 0.8rem;
        background: transparent;
        border: 1px solid #c9a0a0;
        color: #6a2222;
        padding: 0.3rem 0.65rem;
        cursor: pointer;
        letter-spacing: 0.04em;
        flex-shrink: 0;
    }

    .dismiss:hover {
        background: #6a2222;
        color: #fbecec;
        border-color: #6a2222;
    }
</style>
