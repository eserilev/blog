An execution client receives a block today with no information about the state it touches. It must execute the transactions in order, because any transaction can depend on the result of the one before it.

EIP-7928 changes this. Each block carries a **block-level access list** (BAL): every account and storage slot that the block reads or writes, with the values after each transaction.

## What clients gain

- Clients can load state from disk in parallel, before execution starts.
- Transactions with separate access sets can execute in parallel.
- Some sync paths can apply state changes without full re-execution.

## What it costs

The list adds data to every block. Builders must produce it, and clients must make sure that it matches the result of execution. A block with a wrong list is invalid.

## A rough shape

This is not the spec type. It is a simplified version for discussion:

```rust
pub struct AccountChanges {
    pub address: Address,
    pub storage_writes: Vec<SlotWrites>,
    pub storage_reads: Vec<B256>,
    pub balance_changes: Vec<(u16, U256)>,
}
```

## Why it matters

Block validation time is one of the main limits on the gas limit. If clients validate blocks faster, the network can raise the limit safely.
