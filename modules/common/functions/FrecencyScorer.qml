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
    
    property real frecencyBoostFactor: 50
    property real maxFrecencyBoost: 500
    property int termMatchExactBoost: 5000
    property int termMatchPrefixBoost: 3000
    
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
    
    function compareResults(a, b) {
        const aIsExact = a.matchType === root.matchType.EXACT;
        const bIsExact = b.matchType === root.matchType.EXACT;
        
        if (aIsExact !== bIsExact) {
            return aIsExact ? -1 : 1;
        }
        
        if (aIsExact && bIsExact) {
            if (Math.abs(a.frecency - b.frecency) > 1) {
                return b.frecency - a.frecency;
            }
            return b.fuzzyScore - a.fuzzyScore;
        }
        
        if (a.fuzzyScore !== b.fuzzyScore) {
            return b.fuzzyScore - a.fuzzyScore;
        }
        return b.frecency - a.frecency;
    }
    
    function getCombinedScore(fuzzyScore, frecencyBoost) {
        const boost = Math.min(frecencyBoost * root.frecencyBoostFactor, root.maxFrecencyBoost);
        return fuzzyScore + boost;
    }
    
    function getTermMatchBoost(recentTerms, query) {
        const queryLower = query.toLowerCase();
        let boost = 0;
        for (const term of recentTerms) {
            const termLower = term.toLowerCase();
            if (termLower === queryLower) {
                return root.termMatchExactBoost;
            } else if (termLower.startsWith(queryLower)) {
                boost = Math.max(boost, root.termMatchPrefixBoost);
            }
        }
        return boost;
    }
}
