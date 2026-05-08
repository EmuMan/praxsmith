// Hardcoded defaults that auto-fill the editor on load.
// Leave these as empty strings unless you want a built-in starter scenario.

export const DEFAULT_TYPES = `
emotion sad
emotion happy

directional is_in contains

reciprocal is_married_to {
    since: 0..3000
}

practice greet (Greeter, Greeted) {
    actions: [
        {
            for: Greeter
            name: "Greet [Greeted]"
            conditions: [
                $Greeter.is_in.house and $Greeted.is_in.house // TODO: support free variables here
            ]
            outcomes: [
                say "Hello, [Greeted]!"
                set Greeted.feels.happy
                broadcast "[Greeter]'s greeting has made [Greeted] happy!"
                // delete self // TODO: make work
            ]
        }
    ]
}
`;

export const DEFAULT_WORLD = `
agent house as "House"

agent jacob as "Jacob"
agent alaina as "Alaina"

jacob.is_married_to.alaina {
    since: 2024
}

jacob.is_in.house
alaina.is_in.house

jacob.feels.sad

practice.greet.alaina.jacob
`;
