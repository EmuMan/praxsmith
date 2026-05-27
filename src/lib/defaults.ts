// Hardcoded defaults that auto-fill the editor on load.
// Leave these as empty strings unless you want a built-in starter scenario.

export const DEFAULT_TYPES = `
emotion sad
emotion happy

exclusive directional is_in / contains

reciprocal is_married_to {
  since: 0..3000
}

evaluation likes / liked_by

practice greet (Greeter, Greeted) {
  actions: [
    {
      for: Greeter
      name: "Greet [Greeted]"
      conditions: [
        any Place where Greeter.is_in.Place and Greeted.is_in.Place
      ]
      outcomes: [
        say "Hello, [Greeted]!"
        set Greeted.feels.happy
        broadcast "[Greeter]'s greeting has made [Greeted] happy!"
        delete self
      ]
    }
  ]
}

practice move (Actor, From, To) {
  actions: [
    {
      for: Actor
      name: "Move from [From] to [To]"
      conditions: [
        Actor.is_in.From
      ]
      outcomes: [
        set Actor.is_in.To
        broadcast "[Actor] moves from [From] to [To]."
      ]
    }
  ]
}
`;

export const DEFAULT_WORLD = `
actor house as "House" inactive
actor street as "Street" inactive

actor jacob as "Jacob" {
  goal (10): delta count Person where Person.feels.happy
}
actor alaina as "Alaina"

jacob.is_married_to.alaina {
  since: 2024
}

jacob.is_in.house
alaina.is_in.street

alaina.feels.sad

practice.greet.jacob.alaina
practice.move.alaina.street.house
`;
