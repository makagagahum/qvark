# Runtime boundary

Qorx Local Context Resolution is the runtime process of resolving a small
carrier into local evidence under a declared mode and evidence budget.

A carrier can be `.qorx` source, `.qorxb` bytecode, a `qorx://` handle, or a
compact evidence pack. The carrier is not the evidence itself. The Qorx resolver
reads local state and returns a proof page with citations, or it refuses when it
cannot support the request.

The resolver boundary is explicit. Model-visible text stays small. Local state
such as indexes, cache entries, receipts, source files, and provenance records
stays local until the runtime selects evidence for the task.

Q-LCR does not make a remote model know hidden local files from a short handle.
The handle must be routed to a Qorx runtime before evidence can be resolved.
