pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string detectedShell: ""
    property string historyFilePath: ""
    property list<string> entries: []
    property int maxEntries: Config.options?.search?.shellHistory?.maxEntries ?? 500
    property bool ready: false

    property string configuredShell: Config.options?.search?.shellHistory?.shell ?? "auto"
    property string customHistoryPath: Config.options?.search?.shellHistory?.customHistoryPath ?? ""
    property bool enabled: Config.options?.search?.shellHistory?.enable ?? true

    readonly property var preparedEntries: entries.map(cmd => ({
         name: Fuzzy.prepare(cmd),
         command: cmd
     }))

    readonly property int shellHistoryLimit: Config.options.search?.shellHistoryLimit ?? 50

    function fuzzyQuery(search: string): var {
        if (search.trim() === "") {
            return entries.slice(0, shellHistoryLimit); // Return recent commands when no search
        }
        return Fuzzy.go(search, preparedEntries, {
            all: true,
            key: "name",
            limit: shellHistoryLimit
        }).map(r => r.obj.command);
    }

    function fuzzyQueryWithScores(search: string): var {
         if (search.trim() === "") {
             return entries.slice(0, shellHistoryLimit).map((cmd, index) => ({
                 command: cmd,
                 score: 1000 - index * 10
             }));
         }
         return Fuzzy.go(search, preparedEntries, {
             all: true,
             key: "name",
             limit: shellHistoryLimit
         }).map(r => ({
             command: r.obj.command,
             score: r._score
         }));
     }

    function refresh() {
        if (!root.historyFilePath || !root.enabled) return;
        readProc.buffer = [];
        readProc.running = true;
    }

    function resolveHistoryPath(shell: string): string {
         if (root.customHistoryPath) {
             return root.customHistoryPath;
         }

         const home = FileUtils.trimFileProtocol(Directories.home);
         switch (shell) {
             case "zsh":
                 return `${home}/.zsh_history`;
             case "bash":
                 return `${home}/.bash_history`;
             case "fish":
                 return `${home}/.local/share/fish/fish_history`;
             default:
                 return "";
         }
     }

    Component.onCompleted: {
        if (root.enabled) {
            detectShellProc.running = true;
        }
    }

    Process {
         id: detectShellProc
         command: ["bash", "-c", "basename \"$SHELL\""]
         stdout: SplitParser {
             onRead: (line) => {
                 const shellName = line.trim().toLowerCase();
                 
                 if (root.configuredShell !== "auto") {
                     root.detectedShell = root.configuredShell;
                 } else if (shellName.includes("zsh")) {
                     root.detectedShell = "zsh";
                 } else if (shellName.includes("bash")) {
                     root.detectedShell = "bash";
                 } else if (shellName.includes("fish")) {
                     root.detectedShell = "fish";
                 } else {
                     root.detectedShell = "unknown";
                     console.log("[ShellHistory] Unknown shell:", shellName);
                 }
             }
         }
        onExited: (exitCode, exitStatus) => {
            if (root.detectedShell && root.detectedShell !== "unknown") {
                root.historyFilePath = root.resolveHistoryPath(root.detectedShell);

                root.refresh();
            }
        }
    }

    Process {
         id: readProc
         property list<string> buffer: []

         command: {
             const path = root.historyFilePath;
             const limit = root.maxEntries;

             switch (root.detectedShell) {
                 case "zsh":
                     return ["bash", "-c",
                         `cat "${path}" 2>/dev/null | ` +
                         `sed 's/^: [0-9]*:[0-9]*;//' | ` +
                         `tac | ` +
                         `awk '!seen[$0]++' | ` +
                         `head -${limit}`
                     ];
                 case "bash":
                     return ["bash", "-c",
                         `tac "${path}" 2>/dev/null | ` +
                         `awk '!seen[$0]++' | ` +
                         `head -${limit}`
                     ];
                 case "fish":
                     return ["bash", "-c",
                         `grep '^- cmd:' "${path}" 2>/dev/null | ` +
                         `sed 's/^- cmd: //' | ` +
                         `tac | ` +
                         `awk '!seen[$0]++' | ` +
                         `head -${limit}`
                     ];
                 default:
                     return ["echo", ""];
             }
         }

        stdout: SplitParser {
            onRead: (line) => {
                const trimmed = line.trim();
                // Filter out empty lines and very short commands
                if (trimmed && trimmed.length > 1) {
                    readProc.buffer.push(trimmed);
                }
            }
        }

         onExited: (exitCode, exitStatus) => {
             if (exitCode === 0) {
                 root.entries = readProc.buffer;
                 root.ready = true;
             } else {
                 console.error("[ShellHistory] Failed to read history with code", exitCode);
             }
         }
     }

    FileView {
         id: historyFileView
         path: root.historyFilePath
         watchChanges: true
         onFileChanged: {
             delayedRefreshTimer.restart();
         }
     }

    Timer {
         id: delayedRefreshTimer
         interval: 500
         onTriggered: root.refresh()
     }

    IpcHandler {
        target: "shellHistoryService"

        function update(): void {
            root.refresh();
        }
    }
}
