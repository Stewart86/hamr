pragma Singleton

import qs.modules.common
import qs.modules.common.functions
import qs.services
import QtQuick
import Quickshell

Singleton {
    id: root

    // Suggestion weights for different signal types
    readonly property var weights: ({
        time: 0.25,
        day: 0.15,
        workspace: 0.25,
        monitor: 0.15,
        sequence: 0.35,
        session: 0.4,
        runningApps: 0.3,
        streak: 0.1
    })

    // Maximum suggestions to return
    readonly property int maxSuggestions: 2

    // Get smart suggestions based on current context
    // Returns array of { item, confidence, reasons }
    function getSuggestions() {
        if (!HistoryManager.historyLoaded) return [];
        
        const context = ContextTracker.getContext();
        const appItems = HistoryManager.getAppHistoryItems();
        
        if (appItems.length === 0) return [];
        
        const candidates = [];
        
        for (const item of appItems) {
            const result = calculateItemConfidence(item, context, appItems);
            if (result.confidence >= StatisticalUtils.minConfidenceToShow) {
                candidates.push({
                    item: item,
                    confidence: result.confidence,
                    reasons: result.reasons
                });
            }
        }
        
        // Sort by confidence descending
        candidates.sort((a, b) => b.confidence - a.confidence);
        
        // Return top suggestions, ensuring diversity
        return deduplicateSuggestions(candidates).slice(0, maxSuggestions);
    }

    // Calculate confidence score for a single item
    function calculateItemConfidence(item, context, allAppItems) {
        const scores = [];
        const reasons = [];
        const minEvents = StatisticalUtils.minEventsForPattern;
        
        // 1. Time-of-day confidence (±30 min window)
        if (item.hourSlotCounts) {
            const hourCount = item.hourSlotCounts[context.currentHour] ?? 0;
            if (hourCount >= minEvents) {
                const timeScore = StatisticalUtils.wilsonScore(hourCount, item.count);
                if (timeScore > 0.1) {
                    scores.push({ type: 'time', score: timeScore, weight: weights.time });
                    reasons.push(`Often used around ${formatHour(context.currentHour)}`);
                }
            }
        }
        
        // 2. Day-of-week confidence
        if (item.dayOfWeekCounts) {
            const dayCount = item.dayOfWeekCounts[context.currentDay] ?? 0;
            if (dayCount >= minEvents) {
                const dayScore = StatisticalUtils.wilsonScore(dayCount, item.count);
                if (dayScore > 0.1) {
                    scores.push({ type: 'day', score: dayScore, weight: weights.day });
                    reasons.push(`Often used on ${formatDay(context.currentDay)}`);
                }
            }
        }
        
        // 3. Workspace affinity
        if (item.workspaceCounts && context.workspace) {
            const wsCount = item.workspaceCounts[context.workspace] ?? 0;
            if (wsCount >= minEvents) {
                const wsScore = StatisticalUtils.wilsonScore(wsCount, item.count);
                if (wsScore > 0.15) {
                    scores.push({ type: 'workspace', score: wsScore, weight: weights.workspace });
                    reasons.push(`Used on workspace ${context.workspace}`);
                }
            }
        }
        
        // 4. Monitor affinity
        if (item.monitorCounts && context.monitor) {
            const monCount = item.monitorCounts[context.monitor] ?? 0;
            if (monCount >= minEvents) {
                const monScore = StatisticalUtils.wilsonScore(monCount, item.count);
                if (monScore > 0.15) {
                    scores.push({ type: 'monitor', score: monScore, weight: weights.monitor });
                    reasons.push(`Used on ${context.monitor}`);
                }
            }
        }
        
        // 5. Sequence (opened after another app)
        if (item.launchedAfter && context.lastApp) {
            const seqCount = item.launchedAfter[context.lastApp] ?? 0;
            if (seqCount >= minEvents) {
                const lastAppCount = HistoryManager.getAppLaunchCount(context.lastApp);
                const totalLaunches = allAppItems.reduce((sum, a) => sum + a.count, 0);
                
                const seqConfidence = StatisticalUtils.getSequenceConfidence(
                    seqCount, lastAppCount, item.count, totalLaunches
                );
                
                if (seqConfidence > 0.1) {
                    scores.push({ type: 'sequence', score: seqConfidence, weight: weights.sequence });
                    reasons.push(`Often opened after ${context.lastApp}`);
                }
            }
        }
        
        // 6. Session start (first app after login/boot)
        if (context.isSessionStart && item.sessionStartCount >= minEvents) {
            const sessionScore = StatisticalUtils.wilsonScore(item.sessionStartCount, item.count);
            if (sessionScore > 0.15) {
                scores.push({ type: 'session', score: sessionScore, weight: weights.session });
                reasons.push("Usually opened at session start");
            }
        }
        
        // 7. Running apps correlation (other apps currently open)
        if (item.launchedAfter && context.runningApps?.length > 0) {
            let runningAppScore = 0;
            let matchedApp = "";
            
            for (const runningApp of context.runningApps) {
                if (runningApp === item.name) continue;
                
                const coCount = item.launchedAfter[runningApp] ?? 0;
                if (coCount >= minEvents) {
                    const score = StatisticalUtils.wilsonScore(coCount, item.count);
                    if (score > runningAppScore) {
                        runningAppScore = score;
                        matchedApp = runningApp;
                    }
                }
            }
            
            if (runningAppScore > 0.1) {
                scores.push({ type: 'runningApps', score: runningAppScore, weight: weights.runningApps });
                reasons.push(`Often used with ${matchedApp}`);
            }
        }
        
        // 8. Streak bonus (habit detection)
        if (item.consecutiveDays >= 3) {
            const streakScore = Math.min(item.consecutiveDays / 10, 1);
            scores.push({ type: 'streak', score: streakScore, weight: weights.streak });
            reasons.push(`${item.consecutiveDays}-day streak`);
        }
        
        const confidence = StatisticalUtils.calculateCompositeConfidence(scores);
        
        return { confidence, reasons };
    }

    // Remove duplicate suggestions (same app from different signals)
    function deduplicateSuggestions(candidates) {
        const seen = new Set();
        const result = [];
        
        for (const candidate of candidates) {
            if (!seen.has(candidate.item.name)) {
                seen.add(candidate.item.name);
                result.push(candidate);
            }
        }
        
        return result;
    }

    // Format hour for display (e.g., "9am", "2pm")
    function formatHour(hour) {
        if (hour === 0) return "12am";
        if (hour === 12) return "12pm";
        if (hour < 12) return `${hour}am`;
        return `${hour - 12}pm`;
    }

    // Format day for display
    function formatDay(day) {
        const days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
        return days[day] ?? "";
    }

    // Check if suggestions are available (for UI to show/hide suggestion section)
    readonly property bool hasSuggestions: {
        if (!HistoryManager.historyLoaded) return false;
        return getSuggestions().length > 0;
    }

    // Get primary reason for a suggestion (for display)
    function getPrimaryReason(suggestion) {
        if (suggestion.reasons.length === 0) return "Suggested";
        return suggestion.reasons[0];
    }
}
