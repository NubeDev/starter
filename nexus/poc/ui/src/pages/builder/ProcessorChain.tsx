// Edit the ordered list of pipeline processors.

import type { ComponentKind } from "../../api/catalog";
import type { Picked } from "../../builder/assemble";
import { ComponentForm } from "../../components/ComponentForm";

interface Props {
  kinds: ComponentKind[];
  chain: Picked[];
  onChange: (chain: Picked[]) => void;
}

export function ProcessorChain({ kinds, chain, onChange }: Props) {
  function addStep() {
    if (kinds[0]) onChange([...chain, { kind: kinds[0], values: {} }]);
  }
  function setStep(i: number, picked: Picked | null) {
    if (!picked) return onChange(chain.filter((_, idx) => idx !== i));
    onChange(chain.map((p, idx) => (idx === i ? picked : p)));
  }

  return (
    <div className="chain">
      <div className="chain-head">
        <h3>Pipeline processors</h3>
        <button className="ghost" onClick={addStep}>
          + add step
        </button>
      </div>
      {chain.length === 0 && <p className="muted">No processors — input flows straight to output.</p>}
      {chain.map((step, i) => (
        <ComponentForm
          key={i}
          title={`Step ${i + 1}`}
          kinds={kinds}
          picked={step}
          optional
          onChange={(p) => setStep(i, p)}
        />
      ))}
    </div>
  );
}
