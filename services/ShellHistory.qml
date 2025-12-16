pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    // Detected shell type: "zsh", "bash", "fish", or "unknown"
    property string detectedShell: ""
    property string historyFilePath: ""
    property list<string> entries: []
    property int maxEntries: Config.options?.search?.shellHistory?.maxEntries ?? 500
    property bool ready: false

    // Shell config from user settings
    property string configuredShell: Config.options?.search?.shellHistory?.shell ?? "auto"
    property string customHistoryPath: Config.options?.search?.shellHistory?.customHistoryPath ?? ""
    property bool enabled: Config.options?.search?.shellHistory?.enable ?? true

    // Prepared entries for fuzzy search
    readonly property var preparedEntries: entries.map(cmd => ({
        name: Fuzzy.prepare(cmd),
        command: cmd
    }))

    // Shell history result limit from Config
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

    // Fuzzy query with scores for ranking integration
    function fuzzyQueryWithScores(search: string): var {
        if (search.trim() === "") {
            // Return recent commands with position-based scores
            return entries.slice(0, shellHistoryLimit).map((cmd, index) => ({
                command: cmd,
                score: 1000 - index * 10 // More recent = higher score
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

    // Resolve history file path based on shell type
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

    // Initialize on component completion
    Component.onCompleted: {
        if (root.enabled) {
            detectShellProc.running = true;
        }
    }

    // Detect shell type from $SHELL environment variable
    Process {
        id: detectShellProc
        command: ["bash", "-c", "basename \"$SHELL\""]
        stdout: SplitParser {
            onRead: (line) => {
                const shellName = line.trim().toLowerCase();
                
                // Use configured shell if not "auto"
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

    // Read history file with shell-specific parsing
    Process {
        id: readProc
        property list<string> buffer: []

        // Shell-specific parsing commands:
        // - zsh extended format: strip ": TIMESTAMP:DURATION;" prefix
        // - bash: plain text
        // - fish: extract "- cmd: " lines
        // All: reverse order (recent first), deduplicate, limit entries
        command: {
            const path = root.historyFilePath;
            const limit = root.maxEntries;

            switch (root.detectedShell) {
                case "zsh":
                    // Zsh extended history format: ": TIMESTAMP:DURATION;COMMAND"
                    // Also handles plain format (no timestamp)
                    return ["bash", "-c",
                        `cat "${path}" 2>/dev/null | ` +
                        `sed 's/^: [0-9]*:[0-9]*;//' | ` +  // Strip zsh extended format prefix
                        `tac | ` +                          // Reverse (recent first)
                        `awk '!seen[$0]++' | ` +            // Deduplicate (keep first occurrence)
                        `head -${limit}`
                    ];
                case "bash":
                    // Bash: plain text, one command per line
                    return ["bash", "-c",
                        `tac "${path}" 2>/dev/null | ` +
                        `awk '!seen[$0]++' | ` +
                        `head -${limit}`
                    ];
                case "fish":
                    // Fish: YAML-like format with "- cmd: COMMAND"
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

    // Watch for history file changes
    FileView {
        id: historyFileView
        path: root.historyFilePath
        watchChanges: true
        onFileChanged: {
            delayedRefreshTimer.restart();
        }
    }

    // Debounce rapid history file writes
    Timer {
        id: delayedRefreshTimer
        interval: 500
        onTriggered: root.refresh()
    }

    // IPC handler for manual refresh
    IpcHandler {
        target: "shellHistoryService"

        function update(): void {
            root.refresh();
        }
    }
}
