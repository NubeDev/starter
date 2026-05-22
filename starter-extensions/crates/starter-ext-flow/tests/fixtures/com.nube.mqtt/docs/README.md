# com.nube.mqtt

Slice A fixture for the FLOW-NODES track. Contributes two flow node
kinds — `com.nube.mqtt.publish` and `com.nube.mqtt.subscribe` — whose
bodies live in the (out-of-scope-for-slice-A) `bin/mqtt-driver` child
process. Slice A wires only the descriptor surface; the actual proxy
that routes `flow.node.invoke` lands in slice B.
