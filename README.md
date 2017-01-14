# Lͦ ₒm (Loom)
A dynamic layout engine written in Rust using the concept of localized effects.
Content is provided either using `Yarn`-files (similar to Markdown),
or directly from a document graph.

Output formats include PNG (working), PDF (missing), Html (incomplete).

## Status
This is work in progress and unlikely to be usable soon.

## Concept
The whole idea is to reduce complexity as much as possible.
This means logic, that does not interfere with the core is encapsulated
by Traits.

## Contributing
- If you don't love mathematics, this isn't for you.
- If you avoid changing existing code, this isn't your case either.
- Unless you can keep 10kLoC in your head, contributing a module might favourable.

## Modules / Plugins
- input (parsing different input formats into the document graph)
- output (generating other output types)

If the notes and ideas in doc/ make sense to you, ask me on #rust.
