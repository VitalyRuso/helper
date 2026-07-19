# Shared Contracts

Rust structs and JSON schemas shared by the reusable tools and the current app.

The contracts keep review status explicit:

- knowledge candidates are always created as `draft`
- assistant changes are represented as candidates before approval
- runtime assistant config can fall back to the app's existing safe prompt
