-- com.nexus.hello.echo
--
-- Echoes the caller-supplied $message back. Demonstrates that an
-- extension-contributed kind's params validate against its JSON Schema and bind
-- through the shared binder exactly like a file-pack kind — the param is bound,
-- never inlined, so it carries no injection.
SELECT $message AS echoed;
