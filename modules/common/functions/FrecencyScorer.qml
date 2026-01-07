pragma Singleton

import QtQuick
import Quickshell

Singleton {
    id: root
    
    readonly property var matchType: ({
        EXACT: 3,
        PREFIX: 2,
        FUZZY: 1,
        NONE: 0
    })
    
    function getFrecencyScore(historyItem) {
        if (!historyItem) return 0;
        const now = Date.now();
        const hoursSinceUse = (now - historyItem.lastUsed) / (1000 * 60 * 60);
        
        let recencyMultiplier;
        if (hoursSinceUse < 1) recencyMultiplier = 4;
        else if (hoursSinceUse < 24) recencyMultiplier = 2;
        else if (hoursSinceUse < 168) recencyMultiplier = 1;
        else recencyMultiplier = 0.5;
        
        return historyItem.count * recencyMultiplier;
    }
    
    function getMatchType(query, target) {
        if (!query || !target) return root.matchType.NONE;
        const q = query.toLowerCase();
        const t = target.toLowerCase();
        if (t === q) return root.matchType.EXACT;
        if (t.startsWith(q)) return root.matchType.PREFIX;
        return root.matchType.FUZZY;
    }
    
    // Single composite score for efficient sorting (avoids multi-field comparison)
    // fuzzyScore is in 0-1 range (normalized), frecency is typically 0-50
    function getCompositeScore(matchType, fuzzyScore, frecency) {
        // Base score from fuzzy match (0-1 range, scale up for precision)
        let score = fuzzyScore * 1000;
        
        // Match type bonuses (moderate - shouldn't completely override good fuzzy scores)
        if (matchType === root.matchType.EXACT) {
            score += 500;  // Exact history term match
        } else if (matchType === root.matchType.PREFIX) {
            score += 200;
        }
        
        // Frecency bonus (scaled appropriately)
        // frecency typically ranges 0-50 (count * recencyMultiplier)
        // Cap at 300 to not overwhelm fuzzy score differences
        score += Math.min(frecency * 5, 300);
        
        return score;
    }
    
    // Compare using composite scores (faster than multi-field comparison)
    function compareByCompositeScore(a, b) {
        return b.compositeScore - a.compositeScore;
    }

    // Diversity-aware result selection using round-robin interleaving with decay penalty
    // Returns results ordered to maximize diversity while respecting relevance
    function applyDiversity(results, options) {
        const decayFactor = options?.decayFactor ?? 0.7;
        const maxPerSource = options?.maxPerSource ?? Infinity;

        if (results.length === 0) return [];

        // Group results by source (pluginId)
        const bySource = new Map();
        for (const item of results) {
            const sourceId = item._pluginId ?? item.result?._pluginId ?? "__unknown__";
            if (!bySource.has(sourceId)) {
                bySource.set(sourceId, []);
            }
            bySource.get(sourceId).push(item);
        }

        // Sort each source's results by composite score (highest first)
        for (const [sourceId, items] of bySource) {
            items.sort((a, b) => b.compositeScore - a.compositeScore);
        }

        // Track how many items we've taken from each source for decay calculation
        const sourceCounts = new Map();
        for (const sourceId of bySource.keys()) {
            sourceCounts.set(sourceId, 0);
        }

        const diverseResults = [];
        const totalResults = results.length;

        // Round-robin with decay: repeatedly pick the best available item
        // considering decay penalty for sources we've already drawn from
        while (diverseResults.length < totalResults) {
            let bestSource = null;
            let bestEffectiveScore = -Infinity;
            let bestItem = null;

            for (const [sourceId, items] of bySource) {
                if (items.length === 0) continue;

                const count = sourceCounts.get(sourceId);
                
                // Hard limit check
                if (count >= maxPerSource) continue;

                const candidate = items[0];
                const baseScore = candidate.compositeScore;

                // Apply exponential decay: score * (decayFactor ^ count)
                // First item from source: no penalty (decay^0 = 1)
                // Second item: score * 0.7
                // Third item: score * 0.49, etc.
                const effectiveScore = baseScore * Math.pow(decayFactor, count);

                if (effectiveScore > bestEffectiveScore) {
                    bestEffectiveScore = effectiveScore;
                    bestSource = sourceId;
                    bestItem = candidate;
                }
            }

            if (!bestItem) break;

            // Take the best item
            diverseResults.push(bestItem);
            bySource.get(bestSource).shift();
            sourceCounts.set(bestSource, sourceCounts.get(bestSource) + 1);
        }

        return diverseResults;
    }
}
