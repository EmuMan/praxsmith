<script lang="ts">
    let { children } = $props();
</script>

{@render children()}

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
</style>
