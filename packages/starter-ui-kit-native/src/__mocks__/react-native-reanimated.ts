// Vitest-only shim for `react-native-reanimated`. The kit only touches
// `Easing.bezier(...)` to feed `moti`; under tests we return an
// identity function — interpolation maths are not what the snapshots
// are asserting.

type EasingFn = (t: number) => number;

const identity: EasingFn = (t) => t;

export const Easing = {
  bezier(_x1: number, _y1: number, _x2: number, _y2: number): EasingFn {
    return identity;
  },
  linear: identity,
  out(easing: EasingFn): EasingFn {
    return easing;
  },
};
