<script lang="ts">
    import { onMount } from "svelte";
    import {
        EditorView,
        keymap,
        lineNumbers,
        highlightActiveLine,
        highlightActiveLineGutter,
        drawSelection,
        highlightSpecialChars,
        placeholder as placeholderExt,
    } from "@codemirror/view";
    import { EditorState } from "@codemirror/state";
    import {
        defaultKeymap,
        history,
        historyKeymap,
        indentMore,
        indentLess,
        insertNewlineAndIndent,
    } from "@codemirror/commands";
    import { praxsmthExtensions, INDENT_SIZE } from "./praxsmth-lang";

    interface Props {
        value: string;
        placeholder?: string;
    }

    let { value = $bindable(), placeholder }: Props = $props();

    let host: HTMLDivElement;
    let view: EditorView | undefined;

    // Backspace that removes a full indent step when the cursor sits in
    // leading whitespace, falling back to single-char delete otherwise.
    function smartBackspace(target: EditorView): boolean {
        const { state } = target;
        const changes = state.changeByRange((range) => {
            if (!range.empty) return { range }; // let default handle selections
            const line = state.doc.lineAt(range.head);
            const before = state.doc.sliceString(line.from, range.head);
            // Only act when everything before the cursor on this line is spaces.
            if (before.length === 0 || /\S/.test(before)) return { range };
            const col = range.head - line.from;
            const back = ((col - 1) % INDENT_SIZE) + 1;
            const from = range.head - back;
            return {
                changes: { from, to: range.head },
                range: { anchor: from } as any,
            };
        });
        if (changes.changes.empty) return false;
        target.dispatch(state.update(changes, { scrollIntoView: true, userEvent: "delete" }));
        return true;
    }

    onMount(() => {
        const updateListener = EditorView.updateListener.of((u) => {
            if (u.docChanged) value = u.state.doc.toString();
        });

        view = new EditorView({
            parent: host,
            state: EditorState.create({
                doc: value,
                extensions: [
                    lineNumbers(),
                    highlightActiveLineGutter(),
                    highlightActiveLine(),
                    highlightSpecialChars(),
                    drawSelection(),
                    history(),
                    ...(placeholder ? [placeholderExt(placeholder)] : []),
                    ...praxsmthExtensions(),
                    keymap.of([
                        { key: "Backspace", run: smartBackspace },
                        { key: "Enter", run: insertNewlineAndIndent },
                        { key: "Tab", run: indentMore, shift: indentLess },
                        ...defaultKeymap,
                        ...historyKeymap,
                    ]),
                    updateListener,
                    EditorView.theme({
                        "&": { height: "100%" },
                        ".cm-scroller": { overflow: "auto" },
                    }),
                ],
            }),
        });

        return () => view?.destroy();
    });

    // Keep the editor in sync if the value is replaced from outside
    // (e.g. loading a default/example), without clobbering local edits.
    $effect(() => {
        const next = value;
        if (view && next !== view.state.doc.toString()) {
            view.dispatch({
                changes: { from: 0, to: view.state.doc.length, insert: next },
            });
        }
    });
</script>

<div class="cm-host" bind:this={host}></div>

<style>
    .cm-host {
        height: 60vh;
        min-width: 0;
        border: 1px solid #c9bfae;
        background: #fbf7ef;
        overflow: hidden;
    }

    .cm-host :global(.cm-editor) {
        height: 100%;
        font-family:
            "JetBrains Mono", "Fira Code", "SF Mono", "Menlo", "Consolas",
            monospace;
        font-size: 0.85rem;
    }

    .cm-host :global(.cm-editor.cm-focused) {
        outline: none;
    }

    .cm-host:focus-within {
        border-color: #7b7264;
        background: #fffbf3;
    }

    .cm-host :global(.cm-gutters) {
        background: #f3ede1;
        color: #a39684;
        border-right: 1px solid #e0d7c6;
    }

    .cm-host :global(.cm-activeLineGutter) {
        background: #e9e0cf;
        color: #6b4ea0;
    }

    .cm-host :global(.cm-activeLine) {
        background: #f6f0e4;
    }

    .cm-host :global(.cm-content) {
        caret-color: #2a2622;
    }

    .cm-host :global(.cm-cursor) {
        border-left-color: #2a2622;
    }

    .cm-host :global(.cm-selectionBackground),
    .cm-host :global(.cm-focused .cm-selectionBackground) {
        background: #e2d8c4 !important;
    }

    .cm-host :global(.cm-matchingBracket) {
        background: #d8e8d0;
        outline: 1px solid #1f6f5c;
    }
</style>
