pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Wayland

/**
 * Tracks open windows and provides utilities to match them with DesktopEntry apps.
 * 
 * Automatically updates when windows open/close via ToplevelManager reactivity.
 * Groups windows by appId (normalized to lowercase) for efficient lookups.
 */
Singleton {
	id: root

	/**
	 * Map of appId -> array of Toplevel objects
	 * Example: { "firefox": [...toplevels], "code": [...toplevels] }
	 */
	readonly property var appWindows: {
		const map = new Map();
		
		for (const toplevel of ToplevelManager.toplevels.values) {
			const appId = toplevel.appId.toLowerCase();
			
			if (!map.has(appId)) {
				map.set(appId, []);
			}
			
			map.get(appId).push(toplevel);
		}
		
		return map;
	}

	/**
	 * List of all currently open windows
	 */
	readonly property list<var> allWindows: Array.from(ToplevelManager.toplevels.values)

	/**
	 * Total number of open windows
	 */
	readonly property int totalWindowCount: allWindows.length

	/**
	 * Get windows for a specific desktop entry.
	 * 
	 * Matching strategy:
	 * 1. Direct match: entry.id matches normalized appId
	 * 2. Reverse domain extraction: com.microsoft.Edge -> edge, microsoft-edge
	 * 3. Fallback: Use IconResolver substitutions for known mappings
	 * 
	 * @param {string} desktopEntryId - The DesktopEntry id (without .desktop extension)
	 * @returns {list<Toplevel>} Array of open windows for this app
	 */
	function getWindowsForApp(desktopEntryId) {
		if (!desktopEntryId || desktopEntryId.length === 0) {
			return [];
		}

		const normalizedId = desktopEntryId.toLowerCase();

		// Direct match
		if (appWindows.has(normalizedId)) {
			return appWindows.get(normalizedId);
		}

		// Try substitutions from IconResolver
		const substitution = IconResolver.substitutions[desktopEntryId];
		if (substitution && appWindows.has(substitution.toLowerCase())) {
			return appWindows.get(substitution.toLowerCase());
		}

		const substitutionLower = IconResolver.substitutions[normalizedId];
		if (substitutionLower && appWindows.has(substitutionLower.toLowerCase())) {
			return appWindows.get(substitutionLower.toLowerCase());
		}

		// Extract parts from reverse domain name (e.g., com.microsoft.Edge)
		const parts = desktopEntryId.split(".");
		const lastPart = parts[parts.length - 1]?.toLowerCase() ?? "";
		const secondLastPart = parts.length >= 2 ? parts[parts.length - 2]?.toLowerCase() ?? "" : "";
		
		// Try common variations
		const variations = [
			normalizedId,
			normalizedId.replace(/-/g, "_"),
			normalizedId.replace(/_/g, "-"),
			lastPart,  // Edge -> edge
			secondLastPart + "-" + lastPart,  // microsoft-edge
			lastPart + "-" + secondLastPart,  // edge-microsoft (unlikely but try)
			secondLastPart,  // microsoft
		].filter(v => v && v.length > 0);

		for (const variant of variations) {
			if (appWindows.has(variant)) {
				return appWindows.get(variant);
			}
		}

		// Try partial match - check if any appId contains or is contained by our id
		for (const [appId, windows] of appWindows.entries()) {
			// Check if the desktop entry name is contained in the window appId
			if (appId.includes(lastPart) && lastPart.length >= 3) {
				return windows;
			}
			// Check if window appId is contained in desktop entry
			if (normalizedId.includes(appId) && appId.length >= 3) {
				return windows;
			}
		}

		return [];
	}

	/**
	 * Check if a desktop entry has any open windows.
	 * 
	 * @param {string} desktopEntryId - The DesktopEntry id
	 * @returns {bool} True if app has at least one window open
	 */
	function hasRunningWindows(desktopEntryId) {
		return getWindowsForApp(desktopEntryId).length > 0;
	}

	/**
	 * Get the number of open windows for a desktop entry.
	 * 
	 * @param {string} desktopEntryId - The DesktopEntry id
	 * @returns {int} Number of open windows
	 */
	function getWindowCount(desktopEntryId) {
		return getWindowsForApp(desktopEntryId).length;
	}

	/**
	 * Activate and focus a window.
	 * 
	 * @param {Toplevel} toplevel - The window to focus
	 */
	function focusWindow(toplevel) {
		if (!toplevel) return;
		toplevel.activate();
	}

	/**
	 * Request to close a window.
	 * 
	 * @param {Toplevel} toplevel - The window to close
	 */
	function closeWindow(toplevel) {
		if (!toplevel) return;
		toplevel.close();
	}

	/**
	 * Cycle through windows of an app (focus next window in the group).
	 * If the last window is focused, focus the first one.
	 * 
	 * @param {string} desktopEntryId - The DesktopEntry id
	 */
	function cycleWindows(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		if (windows.length === 0) return;

		// Find currently focused window
		let focusedIndex = -1;
		for (let i = 0; i < windows.length; i++) {
			if (windows[i].activated) {
				focusedIndex = i;
				break;
			}
		}

		// Focus next window (or wrap to first)
		const nextIndex = (focusedIndex + 1) % windows.length;
		focusWindow(windows[nextIndex]);
	}

	/**
	 * Get all windows for a given appId (internal use).
	 * 
	 * @param {string} appId - The normalized appId (lowercase)
	 * @returns {list<Toplevel>} Array of windows
	 */
	function getWindowsByAppId(appId) {
		const normalized = appId.toLowerCase();
		return appWindows.has(normalized) ? appWindows.get(normalized) : [];
	}

	/**
	 * Check if any window from this app is currently focused.
	 * 
	 * @param {string} desktopEntryId - The DesktopEntry id
	 * @returns {bool} True if app has a focused window
	 */
	function isAppFocused(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		return windows.some(w => w.activated);
	}

	/**
	 * Get all unique appIds currently running.
	 * 
	 * @returns {list<string>} Array of appIds
	 */
	function getRunningAppIds() {
		return Array.from(appWindows.keys());
	}

	/**
	 * Close all windows for an app.
	 * 
	   * @param {string} desktopEntryId - The DesktopEntry id
	 */
	function closeAllWindowsForApp(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		for (const window of windows) {
			closeWindow(window);
		}
	}
}
