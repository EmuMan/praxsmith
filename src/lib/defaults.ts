// Hardcoded defaults that auto-fill the editor on load.
// Leave these as empty strings unless you want a built-in starter scenario.

export const DEFAULT_TYPES = `
emotion sad
emotion happy

exclusive directional is_in / contains

reciprocal is_married_to {
    since: 0..3000
}

practice greet (Greeter, Greeted) {
    actions: [
        {
            for: Greeter
            name: "Greet [Greeted]"
            conditions: [
                Greeter.is_in.Place and Greeted.is_in.Place
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

practice move (Agent, From, To) {
    actions: [
        {
            for: Agent
            name: "Move from [From] to [To]"
            conditions: [
                Agent.is_in.From
            ]
            outcomes: [
                set Agent.is_in.To
                broadcast "[Agent] moves from [From] to [To]."
            ]
        }
    ]
}
`;

export const DEFAULT_WORLD = `
agent house as "House" inactive
agent street as "Street" inactive

agent jacob as "Jacob"
agent alaina as "Alaina"

jacob.is_married_to.alaina {
    since: 2024
}

jacob.is_in.street
alaina.is_in.house

jacob.feels.sad

practice.greet.alaina.jacob
practice.move.jacob.street.house
`;
