import type { AgentInfo, AvailableAction, Dialog } from "praxsmth";

export type { AgentInfo, AvailableAction, Dialog };

export type ChatMessage =
  | { kind: "system"; line: string }
  | { kind: "speech"; speaker: string; line: string };
