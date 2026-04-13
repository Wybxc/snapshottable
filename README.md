# Snapshottable

A store of mutable references that can be captured and restored efficiently.

A store represents a collection of reference mappings. At any point in time,
the store has a "current mapping". You can create references (`Ref`), update them
(`Ref::set` or `Store::set`), capture the current mapping as a `Snapshot`,
and restore a `Snapshot` later.

## Implementation Principles

The data about all these mappings is internally stored in a *store graph*,
where each node represents a specific snapshot or generation mapping. One
distinguished node in the graph, that we call the *current node* (`Mem`),
represents the current active mapping.

- The actual active values of the current mapping are stored directly inside
  the `Ref` cell instances for efficient read access.
- Edges in the store graph (`Diff` structures) carry information on how to "go back"
  to a previous mapping from another, storing the older value and the reference
  that changed.

When `Store::restore` is called to go back to a captured mapping, the store
traverses the edges from the target snapshot node up to the current node.
On this path, it applies the stored inverse diffs, pulling the old values
back into the references' active memory, while concurrently reversing the
graph pointers to make the snapshot node the new active current node.

## References

1. C. Allain, B. Clément, A. Moine, and G. Scherer, “Snapshottable stores,” Proc, ACM Program, Lang, vol. 8, no. ICFP, p. 248:338-248:369, Aug. 2024, doi: [10.1145/3674637](https://dl.acm.org/doi/10.1145/3674637).
