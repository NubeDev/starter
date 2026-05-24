## Done

- deleted rubix/crates/rubix-tools/src/analytics/weekly_report.rs
- updated rubix/crates/rubix-tools/src/analytics/mod.rs to drop the weekly_report module
- removed the WeeklyReportStub import + Arc::new line from rubix/crates/rubix-agent/src/registry.rs (kept DashboardAssistantStub for Goal 1)
- rewrote rubix/crates/rubix-flows/flows/weekly-report.yaml with trigger: schedule, cron_expr: "0 8 * * 1", allowed_tools = [analytics.query, analytics.report, rubix.alert.send, rubix.undo.last]
- rewrote rubix/crates/rubix-skills/skills/analytics-reporter/SKILL.md in present tense; allowed_tools now match the new four-tool list
- cargo build -p rubix-tools -p rubix-agent -p rubix-flows green
- committed 8fd1fb6 and pushed to origin/codeless/rubix-goal-6-weekly-report

## Next

- Stage 12: phase D.2 — wire FlowAsService at agent boot (per WORKFLOW.md, this is where analytics.query / analytics.report get added to the tool registry and FlowAsService::start is spawned with the scheduled_flows row for com.rubix.weekly-report)

## What you need to know

- I used the tool ids literally as the stage dictated (analytics.query, analytics.report) rather than the `rubix.analytics.*` registration names used by AnalyticsQueryTool/AnalyticsReportTool. The agent registry does not yet register those verbs — Phase D.2 will need to either register them under the bare names the YAML expects, or to translate. Worth a quick check at the start of Stage 12.
- RubixFlowYaml has no formal cron_expr field; serde currently ignores unknown top-level keys, so the YAML parses but cron_expr is dropped on the floor. The scheduler picks cron_expr off the trigger_schedule NodeBehavior config — Phase D.2 may need to either add a cron_expr field to RubixFlowYaml + convert.rs (to seed scheduled_flows from the bundled YAML at boot) or wrap the agent node under a trigger_schedule entry node. I left the literal `cron_expr: "0 8 * * 1"` at the top per the stage spec.
- Registry comment was scrubbed of any phase-marker reference per the "no phasing markers in code" rule.

## Open questions

- (none) — flagged the two integration concerns above for Stage 12 rather than as blockers.
