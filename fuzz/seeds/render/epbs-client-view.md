# ePBS from a client's perspective

Glamsterdam splits the block in two. The proposer commits to a builder bid. The builder reveals the payload later.

## What changes for fork choice

- Fork choice now tracks **payload presence**, not only the block.
- The payload timeliness committee (PTC) votes on whether the payload arrived on time.
- An *empty* slot is now different from a *missed* slot.

## First pass at the type

```rust
pub enum PayloadStatus {
    Pending,
    Revealed { block_hash: Hash256 },
    Withheld,
}
```

> TODO: add the fork choice diagram before publishing.
