import { describe, expect, test } from "bun:test";

// Extract pure functions from StatisticalUtils.qml for testing
const StatisticalUtils = {
  wilsonScore(successes: number, total: number, z = 1.65): number {
    if (total === 0) return 0;

    const p = successes / total;
    const zSquared = z * z;
    const denominator = 1 + zSquared / total;
    const center = p + zSquared / (2 * total);
    const spread =
      z * Math.sqrt((p * (1 - p) + zSquared / (4 * total)) / total);

    return Math.max(0, (center - spread) / denominator);
  },

  sequenceMetrics(
    countAB: number,
    countA: number,
    countB: number,
    totalLaunches: number
  ) {
    if (countA === 0 || totalLaunches === 0) {
      return { support: 0, confidence: 0, lift: 0 };
    }

    const support = countAB / totalLaunches;
    const confidence = countAB / countA;

    const probB = countB / totalLaunches;
    const lift = probB > 0 ? confidence / probB : 0;

    return { support, confidence, lift };
  },

  getSequenceConfidence(
    countAB: number,
    countA: number,
    countB: number,
    totalLaunches: number,
    minCount = 3
  ): number {
    if (countAB < minCount) return 0;

    const metrics = this.sequenceMetrics(countAB, countA, countB, totalLaunches);

    if (metrics.lift < 1.2) return 0;
    if (metrics.confidence < 0.2) return 0;

    return Math.min(metrics.confidence * Math.min(metrics.lift / 2, 1), 1);
  },

  getDecayWeight(timestampMs: number, halfLifeDays = 30): number {
    const now = Date.now();
    const ageMs = now - timestampMs;
    const halfLifeMs = halfLifeDays * 24 * 60 * 60 * 1000;
    const lambda = Math.LN2 / halfLifeMs;

    return Math.exp(-lambda * ageMs);
  },

  isWithinTimeWindow(hourSlot: number, windowMinutes = 30): boolean {
    const now = new Date();
    const currentMinutes = now.getHours() * 60 + now.getMinutes();
    const slotStartMinutes = hourSlot * 60;
    const slotEndMinutes = slotStartMinutes + 60;

    const windowStart = slotStartMinutes - windowMinutes;
    const windowEnd = slotEndMinutes + windowMinutes;

    if (windowStart < 0) {
      return currentMinutes >= windowStart + 1440 || currentMinutes < windowEnd;
    }
    if (windowEnd >= 1440) {
      return currentMinutes >= windowStart || currentMinutes < windowEnd - 1440;
    }

    return currentMinutes >= windowStart && currentMinutes < windowEnd;
  },

  getHourSlot(timestampMs: number): number {
    const date = new Date(timestampMs);
    return date.getHours();
  },

  getDayOfWeek(timestampMs: number): number {
    const date = new Date(timestampMs);
    const jsDay = date.getDay();
    return jsDay === 0 ? 6 : jsDay - 1;
  },

  calculateCompositeConfidence(
    scores: Array<{ score: number; weight: number }>
  ): number {
    if (scores.length === 0) return 0;

    let totalWeight = 0;
    let weightedSum = 0;

    for (const item of scores) {
      if (item.score > 0) {
        totalWeight += item.weight;
        weightedSum += item.score * item.weight;
      }
    }

    return totalWeight > 0 ? weightedSum / totalWeight : 0;
  },

  minConfidenceToShow: 0.3,
  highConfidence: 0.7,
  minEventsForPattern: 3,
};

describe("wilsonScore", () => {
  test("returns 0 for empty total", () => {
    expect(StatisticalUtils.wilsonScore(0, 0)).toBe(0);
    expect(StatisticalUtils.wilsonScore(5, 0)).toBe(0);
  });

  test("returns lower bound for small samples", () => {
    const score = StatisticalUtils.wilsonScore(1, 1);
    expect(score).toBeGreaterThan(0);
    expect(score).toBeLessThan(1);
  });

  test("converges to ratio for large samples", () => {
    const score = StatisticalUtils.wilsonScore(500, 1000);
    expect(score).toBeGreaterThan(0.45);
    expect(score).toBeLessThan(0.55);
  });

  test("handles 0 successes", () => {
    const score = StatisticalUtils.wilsonScore(0, 10);
    expect(score).toBe(0);
  });

  test("is conservative for small samples", () => {
    const smallSample = StatisticalUtils.wilsonScore(3, 5);
    const largeSample = StatisticalUtils.wilsonScore(300, 500);
    expect(smallSample).toBeLessThan(largeSample);
  });
});

describe("sequenceMetrics", () => {
  test("returns zeros for invalid inputs", () => {
    expect(StatisticalUtils.sequenceMetrics(5, 0, 10, 100)).toEqual({
      support: 0,
      confidence: 0,
      lift: 0,
    });
    expect(StatisticalUtils.sequenceMetrics(5, 10, 10, 0)).toEqual({
      support: 0,
      confidence: 0,
      lift: 0,
    });
  });

  test("calculates support correctly", () => {
    const metrics = StatisticalUtils.sequenceMetrics(10, 50, 20, 100);
    expect(metrics.support).toBe(0.1);
  });

  test("calculates confidence correctly", () => {
    const metrics = StatisticalUtils.sequenceMetrics(10, 50, 20, 100);
    expect(metrics.confidence).toBe(0.2);
  });

  test("calculates lift correctly", () => {
    const metrics = StatisticalUtils.sequenceMetrics(10, 50, 20, 100);
    expect(metrics.lift).toBe(1);
  });

  test("lift > 1 indicates positive association", () => {
    const metrics = StatisticalUtils.sequenceMetrics(15, 50, 20, 100);
    expect(metrics.lift).toBeGreaterThan(1);
  });
});

describe("getSequenceConfidence", () => {
  test("returns 0 below minimum count", () => {
    expect(StatisticalUtils.getSequenceConfidence(2, 50, 20, 100)).toBe(0);
  });

  test("returns 0 for low lift", () => {
    expect(StatisticalUtils.getSequenceConfidence(5, 50, 50, 100)).toBe(0);
  });

  test("returns 0 for low confidence", () => {
    expect(StatisticalUtils.getSequenceConfidence(3, 100, 10, 1000)).toBe(0);
  });

  test("returns positive value for strong association", () => {
    const confidence = StatisticalUtils.getSequenceConfidence(20, 30, 25, 100);
    expect(confidence).toBeGreaterThan(0);
  });
});

describe("getDecayWeight", () => {
  test("returns 1 for current timestamp", () => {
    const weight = StatisticalUtils.getDecayWeight(Date.now());
    expect(weight).toBeCloseTo(1, 2);
  });

  test("returns ~0.5 at half-life", () => {
    const halfLifeDays = 30;
    const halfLifeAgo = Date.now() - halfLifeDays * 24 * 60 * 60 * 1000;
    const weight = StatisticalUtils.getDecayWeight(halfLifeAgo, halfLifeDays);
    expect(weight).toBeCloseTo(0.5, 2);
  });

  test("returns small value for old timestamps", () => {
    const veryOld = Date.now() - 365 * 24 * 60 * 60 * 1000;
    const weight = StatisticalUtils.getDecayWeight(veryOld);
    expect(weight).toBeLessThan(0.01);
  });
});

describe("isWithinTimeWindow", () => {
  test("returns true for current hour", () => {
    const currentHour = new Date().getHours();
    expect(StatisticalUtils.isWithinTimeWindow(currentHour)).toBe(true);
  });

  test("handles midnight wraparound", () => {
    const result = StatisticalUtils.isWithinTimeWindow(0, 30);
    expect(typeof result).toBe("boolean");
  });

  test("handles end of day wraparound", () => {
    const result = StatisticalUtils.isWithinTimeWindow(23, 30);
    expect(typeof result).toBe("boolean");
  });
});

describe("getHourSlot", () => {
  test("returns correct hour", () => {
    const noon = new Date();
    noon.setHours(12, 0, 0, 0);
    expect(StatisticalUtils.getHourSlot(noon.getTime())).toBe(12);
  });

  test("returns 0-23 range", () => {
    const hour = StatisticalUtils.getHourSlot(Date.now());
    expect(hour).toBeGreaterThanOrEqual(0);
    expect(hour).toBeLessThanOrEqual(23);
  });
});

describe("getDayOfWeek", () => {
  test("returns Monday as 0", () => {
    const monday = new Date("2025-01-06T12:00:00");
    expect(StatisticalUtils.getDayOfWeek(monday.getTime())).toBe(0);
  });

  test("returns Sunday as 6", () => {
    const sunday = new Date("2025-01-05T12:00:00");
    expect(StatisticalUtils.getDayOfWeek(sunday.getTime())).toBe(6);
  });

  test("returns 0-6 range", () => {
    const day = StatisticalUtils.getDayOfWeek(Date.now());
    expect(day).toBeGreaterThanOrEqual(0);
    expect(day).toBeLessThanOrEqual(6);
  });
});

describe("calculateCompositeConfidence", () => {
  test("returns 0 for empty scores", () => {
    expect(StatisticalUtils.calculateCompositeConfidence([])).toBe(0);
  });

  test("returns weighted average", () => {
    const scores = [
      { score: 0.8, weight: 0.5 },
      { score: 0.4, weight: 0.5 },
    ];
    expect(StatisticalUtils.calculateCompositeConfidence(scores)).toBeCloseTo(
      0.6,
      5
    );
  });

  test("ignores zero scores", () => {
    const scores = [
      { score: 0.8, weight: 0.5 },
      { score: 0, weight: 0.5 },
    ];
    expect(StatisticalUtils.calculateCompositeConfidence(scores)).toBe(0.8);
  });

  test("handles single score", () => {
    const scores = [{ score: 0.7, weight: 0.3 }];
    expect(StatisticalUtils.calculateCompositeConfidence(scores)).toBe(0.7);
  });
});

describe("constants", () => {
  test("minConfidenceToShow is reasonable", () => {
    expect(StatisticalUtils.minConfidenceToShow).toBeGreaterThan(0);
    expect(StatisticalUtils.minConfidenceToShow).toBeLessThan(1);
  });

  test("highConfidence is higher than minConfidenceToShow", () => {
    expect(StatisticalUtils.highConfidence).toBeGreaterThan(
      StatisticalUtils.minConfidenceToShow
    );
  });

  test("minEventsForPattern is positive", () => {
    expect(StatisticalUtils.minEventsForPattern).toBeGreaterThan(0);
  });
});
