-- com.nexus.demo.ping
--
-- A self-contained extension-contributed query-kind: it reads no tenant-scoped
-- table (so the lint requires no $caller_tenant_id predicate) and simply returns
-- a greeting plus the server clock. Proves the WS-14 contribution path —
-- materialise on boot, resolve as the dispatcher's third source, run through the
-- shared binder — without depending on any fixture data.
SELECT 'hello from com.nexus.demo' AS greeting,
       now()                       AS server_time;
