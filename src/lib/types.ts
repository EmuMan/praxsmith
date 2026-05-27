import type { ActorInfo, AvailableAction, Dialog } from "praxsmth";

export type { ActorInfo, AvailableAction, Dialog };

export type ChatMessage =
  | { kind: "system"; line: string }
  | { kind: "speech"; speaker: string; line: string };
