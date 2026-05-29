import * as React from "react";

import { KINDS, RANGES } from "./presets";
import { Field, PillBtn, SegBtn } from "./prims";
import { SavedViewsField } from "./saved-views";
import { IconFilter, IconRange, IconSites, ROLE_ACCENT } from "./icons";

export function FilterRail({
  kindIdx, setKindIdx, rangeIdx, setRangeIdx,
  allHosts, selectedHosts, setSelectedHosts,
  latestSampleMs,
}: {
  kindIdx: number;
  setKindIdx: (n: number) => void;
  rangeIdx: number;
  setRangeIdx: (n: number) => void;
  allHosts: ReadonlyArray<{ uuid: string; name: string }>;
  selectedHosts: ReadonlyArray<string>;
  setSelectedHosts: React.Dispatch<React.SetStateAction<ReadonlyArray<string>>>;
  latestSampleMs: number | null;
}): React.ReactElement {
  const kindPreset = KINDS[kindIdx]!;
  const range = RANGES[rangeIdx]!;
  return (
    <section className="ext-glass p-3 flex flex-wrap items-end gap-x-6 gap-y-3">
      <Field label="Meter type" icon={<IconFilter size={12} />}>
        <div className="flex gap-1">
          {KINDS.map((k, i) => {
            const KindIcon = ROLE_ACCENT[k.kind].Icon;
            return (
              <SegBtn key={k.kind} active={i === kindIdx} onClick={() => setKindIdx(i)} title={k.hint}>
                <span className="inline-flex items-center gap-1.5">
                  <KindIcon size={12} />
                  {k.label}
                </span>
              </SegBtn>
            );
          })}
        </div>
        <span className="ext-eyebrow mt-1 block normal-case tracking-normal text-muted-foreground">
          {kindPreset.hint}
        </span>
      </Field>

      <Field label="Range" icon={<IconRange size={12} />}>
        <div className="flex gap-1">
          {RANGES.map((r, i) => (
            <SegBtn key={r.label} active={i === rangeIdx} onClick={() => setRangeIdx(i)}>
              {r.label}
            </SegBtn>
          ))}
        </div>
        <span className="ext-eyebrow mt-1 block normal-case tracking-normal text-muted-foreground">
          bucket = {range.bucket}
          {latestSampleMs ? <> · anchor {new Date(latestSampleMs).toLocaleDateString()}</> : null}
        </span>
      </Field>

      <Field label={`Sites · ${selectedHosts.length}/${allHosts.length}`} icon={<IconSites size={12} />}>
        {/* The Regions roll-up + portfolio table below already
            provide rich site selection. Keep the rail compact with
            just the two bulk actions; full per-site control lives
            in those scaled surfaces. */}
        <div className="flex flex-wrap gap-1">
          <PillBtn
            active={selectedHosts.length === allHosts.length && allHosts.length > 0}
            onClick={() => setSelectedHosts(allHosts.map((h) => h.uuid))}
          >
            all
          </PillBtn>
          <PillBtn
            active={selectedHosts.length === 0}
            onClick={() => setSelectedHosts([])}
          >
            none
          </PillBtn>
          <PillBtn onClick={() => setSelectedHosts((sel) => {
            const cur = new Set(sel);
            return allHosts.map((h) => h.uuid).filter((u) => !cur.has(u));
          })}>
            invert
          </PillBtn>
        </div>
        <span className="ext-eyebrow mt-1 block normal-case tracking-normal text-muted-foreground">
          use regions or table below
        </span>
      </Field>

      <SavedViewsField
        kindIdx={kindIdx}
        rangeIdx={rangeIdx}
        selectedHosts={selectedHosts}
        allHosts={allHosts}
        onApply={(v) => {
          setKindIdx(v.kindIdx);
          setRangeIdx(v.rangeIdx);
          setSelectedHosts(v.selectedHosts);
        }}
      />
    </section>
  );
}
