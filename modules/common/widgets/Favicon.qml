import qs.modules.common
import qs.modules.common.widgets
import qs.services
import qs.modules.common.functions
import Qt5Compat.GraphicalEffects
import QtQuick
import Quickshell.Io
import Quickshell.Widgets

IconImage {
    id: root
    property string url: ""
    property string displayText: ""

    property real size: 32
    readonly property string downloadUserAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"
    readonly property string faviconDownloadPath: Directories.favicons ?? ""
    readonly property string domainName: {
        if (!url || url.length === 0) return ""
        if (url.includes("vertexaisearch")) return displayText.length > 0 ? displayText : ""
        const domain = StringUtils.getDomain(url)
        return domain ? domain : ""
    }
    readonly property bool hasDomain: domainName.length > 0 && faviconDownloadPath.length > 0
    readonly property string faviconUrl: hasDomain ? `https://www.google.com/s2/favicons?domain=${domainName}&sz=32` : ""
    readonly property string fileName: hasDomain ? `${domainName}.ico` : ""
    readonly property string faviconFilePath: hasDomain ? `${faviconDownloadPath}/${fileName}` : ""
    
    property string currentSource: ""
    property int loadAttempt: 0

    Process {
        id: faviconDownloadProcess
        running: false
        command: root.hasDomain ? [
            "sh", "-c",
            `curl -s '${root.faviconUrl}' -o '${root.faviconFilePath}' -L -H 'User-Agent: ${root.downloadUserAgent}' && file '${root.faviconFilePath}' | grep -q image`
        ] : []
        onExited: (exitCode, exitStatus) => {
            if (exitCode === 0) {
                root.loadAttempt = 2
                reloadTimer.restart()
            } else {
                root.loadAttempt = 3
                root.currentSource = ""
            }
        }
    }
    
    Timer {
        id: reloadTimer
        interval: 50
        onTriggered: {
            root.currentSource = Qt.resolvedUrl(root.faviconFilePath)
        }
    }

    Component.onCompleted: {
        if (root.hasDomain) {
            root.loadAttempt = 1
            root.currentSource = Qt.resolvedUrl(root.faviconFilePath)
        }
    }

    onStatusChanged: {
        if (status === Image.Error && root.loadAttempt === 1 && root.hasDomain) {
            root.currentSource = ""
            faviconDownloadProcess.running = true
        } else if (status === Image.Error && root.loadAttempt === 2) {
            root.loadAttempt = 3
            root.currentSource = ""
        }
    }

    source: currentSource
    implicitSize: root.size

    layer.enabled: true
    layer.effect: OpacityMask {
        maskSource: Rectangle {
            width: root.implicitSize
            height: root.implicitSize
            radius: Appearance.rounding.full
        }
    }
}
