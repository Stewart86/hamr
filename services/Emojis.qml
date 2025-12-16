pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io

/**
 * Emojis service - provides emoji data for fuzzy search.
 * Loads from bundled plugins/emoji/emojis.txt file.
 */
Singleton {
    id: root
    
    // Bundled emoji data in plugin
    readonly property string bundledEmojisPath: `${Directories.builtinPlugins}/emoji/emojis.txt`
    
    property list<var> list
    readonly property var preparedEntries: list.map(a => ({
        name: Fuzzy.prepare(`${a}`),
        entry: a
    }))
    
    function fuzzyQuery(search: string): var {
        return Fuzzy.go(search, preparedEntries, {
            all: true,
            key: "name"
        }).map(r => r.obj.entry);
    }

    function load() {
        emojiFileView.reload()
    }

    // Load from bundled emoji plugin data
    FileView { 
        id: emojiFileView
        path: Qt.resolvedUrl(root.bundledEmojisPath)
        onLoadedChanged: {
            if (loaded) {
                const fileContent = emojiFileView.text()
                const lines = fileContent.split("\n").filter(line => line.trim() !== "")
                root.list = lines.map(line => line.trim())
            }
        }
    }
}
