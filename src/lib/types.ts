import type { AgentInfo, Dialog } from "praxsmth";

export type { AgentInfo, Dialog };

export type ChatMessage =
    | { kind: "system"; line: string }
    | { kind: "speech"; speaker: string; line: string };
