pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Wayland

Singleton {
	id: root

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

	readonly property list<var> allWindows: Array.from(ToplevelManager.toplevels.values)

	readonly property int totalWindowCount: allWindows.length

	function getWindowsForApp(desktopEntryId) {
		if (!desktopEntryId || desktopEntryId.length === 0) {
			return [];
		}

		const normalizedId = desktopEntryId.toLowerCase();

		if (appWindows.has(normalizedId)) {
			return appWindows.get(normalizedId);
		}

		const substitution = IconResolver.substitutions[desktopEntryId];
		if (substitution && appWindows.has(substitution.toLowerCase())) {
			return appWindows.get(substitution.toLowerCase());
		}

		const substitutionLower = IconResolver.substitutions[normalizedId];
		if (substitutionLower && appWindows.has(substitutionLower.toLowerCase())) {
			return appWindows.get(substitutionLower.toLowerCase());
		}

		const parts = desktopEntryId.split(".");
		const lastPart = parts[parts.length - 1]?.toLowerCase() ?? "";
		const secondLastPart = parts.length >= 2 ? parts[parts.length - 2]?.toLowerCase() ?? "" : "";
		
		const variations = [
			normalizedId,
			normalizedId.replace(/-/g, "_"),
			normalizedId.replace(/_/g, "-"),
			lastPart,
			secondLastPart + "-" + lastPart,
			lastPart + "-" + secondLastPart,
			secondLastPart,
		].filter(v => v && v.length > 0);

		for (const variant of variations) {
			if (appWindows.has(variant)) {
				return appWindows.get(variant);
			}
		}

		for (const [appId, windows] of appWindows.entries()) {
			if (appId.includes(lastPart) && lastPart.length >= 3) {
				return windows;
			}
			if (normalizedId.includes(appId) && appId.length >= 3) {
				return windows;
			}
		}

		return [];
	}

	function hasRunningWindows(desktopEntryId) {
		return getWindowsForApp(desktopEntryId).length > 0;
	}

	function getWindowCount(desktopEntryId) {
		return getWindowsForApp(desktopEntryId).length;
	}

	function focusWindow(toplevel) {
		if (!toplevel) return;
		toplevel.activate();
	}

	function closeWindow(toplevel) {
		if (!toplevel) return;
		toplevel.close();
	}

	function cycleWindows(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		if (windows.length === 0) return;

		let focusedIndex = -1;
		for (let i = 0; i < windows.length; i++) {
			if (windows[i].activated) {
				focusedIndex = i;
				break;
			}
		}

		const nextIndex = (focusedIndex + 1) % windows.length;
		focusWindow(windows[nextIndex]);
	}

	function getWindowsByAppId(appId) {
		const normalized = appId.toLowerCase();
		return appWindows.has(normalized) ? appWindows.get(normalized) : [];
	}

	function isAppFocused(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		return windows.some(w => w.activated);
	}

	function getRunningAppIds() {
		return Array.from(appWindows.keys());
	}

	function closeAllWindowsForApp(desktopEntryId) {
		const windows = getWindowsForApp(desktopEntryId);
		for (const window of windows) {
			closeWindow(window);
		}
	}
}
