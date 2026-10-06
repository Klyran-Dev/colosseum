// Material 3 Expressive spring tokens as CSS (ADR-0422).
//
// Damping ratio / stiffness pairs are the published Material defaults. A unit
// mass spring's closed-form step response is sampled into a CSS linear()
// easing so the web renders the same physics without a JS animation runtime.
// Spatial springs (ratio 0.9) overshoot slightly; effects springs (ratio 1.0)
// never overshoot, so colour/opacity never leave their valid range.
export const SPRINGS = {
  "fast-spatial": { damping: 0.9, stiffness: 1400 },
  "default-spatial": { damping: 0.9, stiffness: 700 },
  "slow-spatial": { damping: 0.9, stiffness: 300 },
  "fast-effects": { damping: 1, stiffness: 3800 },
  "default-effects": { damping: 1, stiffness: 1600 },
  "slow-effects": { damping: 1, stiffness: 800 },
};

const SETTLE_TOLERANCE = 0.001;
const SAMPLES = 24;

function position(time, damping, stiffness) {
  const omega = Math.sqrt(stiffness);
  if (damping >= 1) return 1 - Math.exp(-omega * time) * (1 + omega * time);
  const damped = omega * Math.sqrt(1 - damping * damping);
  const decay = Math.exp(-damping * omega * time);
  return 1 - decay * (Math.cos(damped * time) + ((damping * omega) / damped) * Math.sin(damped * time));
}

export function springToken({ damping, stiffness }) {
  let settle = 0;
  for (let time = 0; time < 5; time += 0.001) {
    if (Math.abs(1 - position(time, damping, stiffness)) > SETTLE_TOLERANCE) settle = time;
  }
  const duration = Math.ceil((settle + 0.001) * 1000);
  const points = Array.from({ length: SAMPLES + 1 }, (_, index) => {
    const value = index === SAMPLES ? 1 : position((index / SAMPLES) * (duration / 1000), damping, stiffness);
    return String(Math.round(value * 1000) / 1000);
  });
  points[0] = "0";
  return { duration: `${duration}ms`, easing: `linear(${points.join(", ")})` };
}

export function springTokens() {
  return Object.fromEntries(Object.entries(SPRINGS).map(([name, spring]) => [name, springToken(spring)]));
}
