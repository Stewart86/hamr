pragma Singleton

import QtQuick
import Quickshell

Singleton {
    id: root

    // Wilson Score Interval (lower bound)
    // Better than simple success/total for small samples
    // z = 1.96 for 95% confidence, 1.645 for 90%
    function wilsonScore(successes, total, z = 1.65) {
        if (total === 0) return 0;
        
        const p = successes / total;
        const zSquared = z * z;
        const denominator = 1 + zSquared / total;
        const center = p + zSquared / (2 * total);
        const spread = z * Math.sqrt((p * (1 - p) + zSquared / (4 * total)) / total);
        
        return Math.max(0, (center - spread) / denominator);
    }

    // Time-weighted Wilson score with exponential decay
    // halfLifeDays: after this many days, weight = 0.5
    function timeWeightedWilsonScore(slotCounts, currentSlot, totalCount, halfLifeDays = 60) {
        if (totalCount === 0) return 0;
        
        const slotCount = slotCounts[currentSlot] ?? 0;
        if (slotCount === 0) return 0;
        
        return wilsonScore(slotCount, totalCount);
    }

    // Association rule metrics for sequence detection
    // "After opening appA, user opens appB"
    function sequenceMetrics(countAB, countA, countB, totalLaunches) {
        if (countA === 0 || totalLaunches === 0) {
            return { support: 0, confidence: 0, lift: 0 };
        }
        
        const support = countAB / totalLaunches;
        const confidence = countAB / countA;
        
        const probB = countB / totalLaunches;
        const lift = probB > 0 ? confidence / probB : 0;
        
        return { support, confidence, lift };
    }

    // Check if sequence association is significant
    // Returns confidence score (0-1) if significant, 0 otherwise
    function getSequenceConfidence(countAB, countA, countB, totalLaunches, minCount = 3) {
        if (countAB < minCount) return 0;
        
        const metrics = sequenceMetrics(countAB, countA, countB, totalLaunches);
        
        if (metrics.lift < 1.2) return 0;
        if (metrics.confidence < 0.2) return 0;
        
        return Math.min(metrics.confidence * Math.min(metrics.lift / 2, 1), 1);
    }

    // Exponential decay weight for time-based signals
    // Returns weight between 0 and 1
    function getDecayWeight(timestampMs, halfLifeDays = 30) {
        const now = Date.now();
        const ageMs = now - timestampMs;
        const halfLifeMs = halfLifeDays * 24 * 60 * 60 * 1000;
        const lambda = Math.LN2 / halfLifeMs;
        
        return Math.exp(-lambda * ageMs);
    }

    // Check if current hour is within ±30 min window of a slot
    // hourSlot: 0-23
    // Returns true if current time is within the window
    function isWithinTimeWindow(hourSlot, windowMinutes = 30) {
        const now = new Date();
        const currentMinutes = now.getHours() * 60 + now.getMinutes();
        const slotStartMinutes = hourSlot * 60;
        const slotEndMinutes = slotStartMinutes + 60;
        
        const windowStart = slotStartMinutes - windowMinutes;
        const windowEnd = slotEndMinutes + windowMinutes;
        
        if (windowStart < 0) {
            return currentMinutes >= (windowStart + 1440) || currentMinutes < windowEnd;
        }
        if (windowEnd >= 1440) {
            return currentMinutes >= windowStart || currentMinutes < (windowEnd - 1440);
        }
        
        return currentMinutes >= windowStart && currentMinutes < windowEnd;
    }

    // Get the hour slot for a given timestamp
    function getHourSlot(timestampMs) {
        const date = new Date(timestampMs);
        return date.getHours();
    }

    // Get the day of week (0=Monday, 6=Sunday) for a given timestamp
    function getDayOfWeek(timestampMs) {
        const date = new Date(timestampMs);
        const jsDay = date.getDay();
        return jsDay === 0 ? 6 : jsDay - 1;
    }

    // Calculate consecutive day streak
    // Returns number of consecutive days the item was used up to today
    function calculateStreak(lastConsecutiveDate, lastUsed) {
        if (!lastConsecutiveDate) return 0;
        
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        const todayStr = today.toISOString().split('T')[0];
        
        const yesterday = new Date(today);
        yesterday.setDate(yesterday.getDate() - 1);
        const yesterdayStr = yesterday.toISOString().split('T')[0];
        
        if (lastConsecutiveDate === todayStr) {
            return 1;
        } else if (lastConsecutiveDate === yesterdayStr) {
            return 1;
        }
        
        return 0;
    }

    // Composite confidence score combining multiple signals
    // weights: { time: 0.25, day: 0.15, workspace: 0.2, monitor: 0.15, sequence: 0.3, session: 0.35 }
    function calculateCompositeConfidence(scores) {
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
    }

    // Confidence thresholds
    readonly property real minConfidenceToShow: 0.3
    readonly property real highConfidence: 0.7
    readonly property int minEventsForPattern: 3
}
