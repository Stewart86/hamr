import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import Qt.labs.synchronizer
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland

Scope {
    id: launcherScope
    property bool dontAutoCancelSearch: false

    Variants {
        id: launcherVariants
        model: Quickshell.screens
        PanelWindow {
            id: root
            required property var modelData
            property string searchingText: ""
            readonly property HyprlandMonitor monitor: Hyprland.monitorFor(root.screen)
            property bool monitorIsFocused: (Hyprland.focusedMonitor?.id == monitor?.id)
            screen: modelData
            visible: GlobalStates.launcherOpen && monitorIsFocused

            WlrLayershell.namespace: "quickshell:hamr"
            WlrLayershell.layer: WlrLayer.Overlay
            color: "transparent"

            mask: Region {
                // Accept input on the full screen background for click-outside-to-close
                item: GlobalStates.launcherOpen ? fullScreenBackground : null
            }

            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }

            HyprlandFocusGrab {
                id: grab
                windows: [root]
                property bool canBeActive: root.monitorIsFocused
                active: false
                onCleared: {
                    // Re-grab focus if launcher should still be open.
                    // Do not re-grab while the ImageBrowser overlay is open,
                    // otherwise it can immediately get its grab cleared and close.
                    if (GlobalStates.launcherOpen && canBeActive && !GlobalStates.imageBrowserOpen) {
                        delayedGrabTimer.restart();
                    }
                }
            }

            Connections {
                target: GlobalStates
                function onLauncherOpenChanged() {
                    if (!GlobalStates.launcherOpen) {
                        searchWidget.disableExpandAnimation();
                        launcherScope.dontAutoCancelSearch = false;
                        grab.active = false;
                        // Clear workflow state when closing
                        if (WorkflowRunner.isActive()) {
                            LauncherSearch.closeWorkflow();
                        }
                    } else {
                        if (!launcherScope.dontAutoCancelSearch) {
                            searchWidget.cancelSearch();
                        }
                        if (!GlobalStates.imageBrowserOpen) {
                            delayedGrabTimer.start();
                        }
                        // Refresh workflows to detect newly added ones (workaround for symlink detection)
                        WorkflowRunner.refreshWorkflows();
                    }
                }
            }

            // Handle workflow execute with close
            Connections {
                target: WorkflowRunner
                function onExecuteCommand(command) {
                    if (command.close) {
                        LauncherSearch.closeWorkflow();
                        GlobalStates.launcherOpen = false;
                    }
                }
            }

            // Pause launcher focus grab while ImageBrowser is open
            Connections {
                target: GlobalStates
                function onImageBrowserOpenChanged() {
                    if (GlobalStates.imageBrowserOpen) {
                        grab.active = false;
                    } else if (GlobalStates.launcherOpen) {
                        delayedGrabTimer.start();
                    }
                }
            }

            Timer {
                id: delayedGrabTimer
                interval: 20 // Race condition delay
                repeat: false
                onTriggered: {
                    if (!grab.canBeActive)
                        return;
                    if (GlobalStates.imageBrowserOpen)
                        return;
                    grab.active = GlobalStates.launcherOpen;
                }
            }

            implicitWidth: columnLayout.implicitWidth
            implicitHeight: columnLayout.implicitHeight

            function setSearchingText(text) {
                searchWidget.setSearchingText(text);
                searchWidget.focusFirstItem();
            }

            // Full screen background Item for mask and click detection
            Item {
                id: fullScreenBackground
                anchors.fill: parent

                // Background MouseArea to close when clicking outside search widget
                MouseArea {
                    anchors.fill: parent
                    visible: GlobalStates.launcherOpen
                    onClicked: (mouse) => {
                        // Check if click is outside the search widget content
                        const content = searchWidget.contentItem;
                        const mapped = content.mapToItem(fullScreenBackground, 0, 0);

                        if (mouse.x < mapped.x || mouse.x > mapped.x + content.width ||
                            mouse.y < mapped.y || mouse.y > mapped.y + content.height) {
                            // Click is outside - close
                            if (WorkflowRunner.isActive()) {
                                LauncherSearch.closeWorkflow();
                            }
                            GlobalStates.launcherOpen = false;
                        }
                    }
                }
            }

            Column {
                id: columnLayout
                visible: GlobalStates.launcherOpen
                anchors {
                    horizontalCenter: parent.horizontalCenter
                    top: parent.top
                }
                spacing: -8

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Escape) {
                        // If workflow is active, exit workflow first; otherwise close launcher
                        if (WorkflowRunner.isActive()) {
                            LauncherSearch.exitWorkflow();
                        } else {
                            GlobalStates.launcherOpen = false;
                        }
                    }
                }

                SearchWidget {
                    id: searchWidget
                    anchors.horizontalCenter: parent.horizontalCenter
                    Synchronizer on searchingText {
                        property alias source: root.searchingText
                    }
                }
            }
        }
    }

    function toggleClipboard() {
        if (GlobalStates.launcherOpen && launcherScope.dontAutoCancelSearch) {
            GlobalStates.launcherOpen = false;
            return;
        }
        for (let i = 0; i < launcherVariants.instances.length; i++) {
            let panelWindow = launcherVariants.instances[i];
            if (panelWindow.modelData.name == Hyprland.focusedMonitor.name) {
                launcherScope.dontAutoCancelSearch = true;
                panelWindow.setSearchingText(Config.options.search.prefix.clipboard);
                GlobalStates.launcherOpen = true;
                return;
            }
        }
    }

    function toggleEmojis() {
        if (GlobalStates.launcherOpen && launcherScope.dontAutoCancelSearch) {
            GlobalStates.launcherOpen = false;
            return;
        }
        for (let i = 0; i < launcherVariants.instances.length; i++) {
            let panelWindow = launcherVariants.instances[i];
            if (panelWindow.modelData.name == Hyprland.focusedMonitor.name) {
                launcherScope.dontAutoCancelSearch = true;
                panelWindow.setSearchingText(Config.options.search.prefix.emojis);
                GlobalStates.launcherOpen = true;
                return;
            }
        }
    }
}
