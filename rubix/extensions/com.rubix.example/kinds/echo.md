# `com.rubix.example.echo`

Reference echo contribution.

The **tool** flavour returns the input `{ message }` object verbatim,
demonstrating the minimum a `starter-ext-sdk`-based extension needs to
ship to expose an MCP tool to the host.

The **flow-node** flavour declares the same id under `contributes.nodes`
so the rubix flow editor surfaces it in the node palette. Today its
runtime behaviour is provided by the upstream slice-A placeholder
binding (`NodeError::Domain { code: "no_behaviour_bound" }`); slice-B's
`ProcessNodeProxy` will route `flow.node.invoke` to the same child
binary that already serves the tool.
