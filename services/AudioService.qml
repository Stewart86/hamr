pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import qs.modules.common

Singleton {
    id: root

    property bool available: false
    readonly property bool enabled: Config.options.audio?.enabled ?? true
    property var soundPlayers: ({})
    property var soundFilePaths: ({})

    readonly property var soundEvents: [
        "alarm",
        "timer",
        "complete",
        "notification",
        "error",
        "warning"
    ]

    readonly property var soundEventMap: ({
        "alarm": ["alarm-clock-elapsed", "bell"],
        "timer": ["alarm-clock-elapsed", "complete"],
        "complete": ["complete", "message"],
        "notification": ["message", "message-new-instant"],
        "error": ["dialog-error", "bell"],
        "warning": ["dialog-warning", "bell"]
    })

    Component.onCompleted: {
        detectAvailability();
    }

    function detectAvailability() {
        try {
            const testObj = Qt.createQmlObject(`
                import QtQuick
                import QtMultimedia
                Item {}
            `, root, "AudioService.TestComponent");
            if (testObj) {
                testObj.destroy();
            }
            available = true;
            console.info("AudioService: QtMultimedia available");
            discoverSoundFiles();
        } catch (e) {
            available = false;
            console.warn("AudioService: QtMultimedia not available - sound effects disabled");
        }
    }

    function discoverSoundFiles() {
        const userSoundsDir = Directories.hamrConfig + "/sounds";
        discoverProcess.userSoundsDir = userSoundsDir;
        discoverProcess.running = true;
    }

    Process {
        id: discoverProcess
        property string userSoundsDir: ""
        property var discoveredPaths: ({})

        command: ["sh", "-c", `
            user_dir="${userSoundsDir}"

            # System sound directories in priority order (modern themes first)
            system_dirs="/usr/share/sounds/ocean/stereo /usr/share/sounds/freedesktop/stereo"

            for event in alarm timer complete notification error warning; do
                found=0

                case "$event" in
                    alarm)
                        names="alarm alarm-clock-elapsed bell"
                        ;;
                    timer)
                        names="timer alarm-clock-elapsed completion-success complete"
                        ;;
                    complete)
                        names="complete completion-success outcome-success message dialog-information"
                        ;;
                    notification)
                        names="notification message-new-instant message-highlight message"
                        ;;
                    error)
                        names="error dialog-error completion-fail outcome-failure bell"
                        ;;
                    warning)
                        names="warning dialog-warning dialog-warning-auth bell"
                        ;;
                esac

                # Check user sounds first
                if [ -d "$user_dir" ]; then
                    for name in $names; do
                        for ext in oga ogg wav mp3 flac; do
                            file_path="$user_dir/$name.$ext"
                            if [ -f "$file_path" ]; then
                                echo "$event=$file_path"
                                found=1
                                break
                            fi
                        done
                        [ $found -eq 1 ] && break
                    done
                fi

                # Fall back to system sounds
                if [ $found -eq 0 ]; then
                    for system_dir in $system_dirs; do
                        [ -d "$system_dir" ] || continue
                        for name in $names; do
                            for ext in oga ogg wav mp3 flac; do
                                file_path="$system_dir/$name.$ext"
                                if [ -f "$file_path" ]; then
                                    echo "$event=$file_path"
                                    found=1
                                    break
                                fi
                            done
                            [ $found -eq 1 ] && break
                        done
                        [ $found -eq 1 ] && break
                    done
                fi
            done
        `]

        stdout: SplitParser {
            onRead: line => {
                const parts = line.split('=');
                if (parts.length === 2) {
                    discoverProcess.discoveredPaths[parts[0]] = "file://" + parts[1];
                }
            }
        }

        onExited: (code, status) => {
            if (code === 0) {
                root.soundFilePaths = discoverProcess.discoveredPaths;
                root.createSoundPlayers();
            }
            discoverProcess.discoveredPaths = {};
        }
    }

    function createSoundPlayers() {
        if (!available) return;

        destroySoundPlayers();

        for (const event of soundEvents) {
            const soundPath = soundFilePaths[event];
            if (!soundPath) continue;

            try {
                const player = Qt.createQmlObject(`
                    import QtQuick
                    import QtMultimedia
                    MediaPlayer {
                        source: "${soundPath}"
                        audioOutput: AudioOutput { volume: 1.0 }
                    }
                `, root, `AudioService.${event}Sound`);

                if (player) {
                    soundPlayers[event] = player;
                }
            } catch (e) {
                console.warn(`AudioService: Failed to create player for ${event}:`, e);
            }
        }
    }

    function destroySoundPlayers() {
        for (const event in soundPlayers) {
            if (soundPlayers[event]) {
                soundPlayers[event].destroy();
            }
        }
        soundPlayers = {};
    }

    function playSound(soundOrPath: string): bool {
        if (!available || !enabled) return false;

        // Check if it's a predefined sound event
        if (soundPlayers[soundOrPath]) {
            soundPlayers[soundOrPath].play();
            return true;
        }

        // Check if it's a path (absolute or file://)
        if (soundOrPath.startsWith("/") || soundOrPath.startsWith("file://")) {
            return playCustomSound(soundOrPath);
        }

        // Check user sounds directory for custom name
        const userPath = Directories.hamrConfig + "/sounds/" + soundOrPath;
        return playCustomSound(userPath);
    }

    function playCustomSound(path: string): bool {
        if (!available || !enabled) return false;

        const sourcePath = path.startsWith("file://") ? path : "file://" + path;

        try {
            const player = Qt.createQmlObject(`
                import QtQuick
                import QtMultimedia
                MediaPlayer {
                    id: customPlayer
                    source: "${sourcePath}"
                    audioOutput: AudioOutput { volume: 1.0 }
                    onPlaybackStateChanged: {
                        if (playbackState === MediaPlayer.StoppedState) {
                            customPlayer.destroy();
                        }
                    }
                    Component.onCompleted: play()
                }
            `, root, "AudioService.CustomSound");

            return player !== null;
        } catch (e) {
            console.warn("AudioService: Failed to play custom sound:", path, e);
            return false;
        }
    }

    function playAlarm() { return playSound("alarm"); }
    function playTimer() { return playSound("timer"); }
    function playComplete() { return playSound("complete"); }
    function playNotification() { return playSound("notification"); }
    function playError() { return playSound("error"); }
    function playWarning() { return playSound("warning"); }

    IpcHandler {
        target: "audio"

        function play(sound: string): string {
            if (root.playSound(sound)) {
                return `Playing: ${sound}`;
            }
            return `Failed to play: ${sound}`;
        }

        function enable(): string {
            Config.setNestedValue("audio.enabled", true);
            return "Audio enabled";
        }

        function disable(): string {
            Config.setNestedValue("audio.enabled", false);
            return "Audio disabled";
        }

        function status(): string {
            return JSON.stringify({
                available: root.available,
                enabled: root.enabled,
                sounds: Object.keys(root.soundFilePaths)
            }, null, 2);
        }

        function reload(): string {
            root.discoverSoundFiles();
            return "Reloading sounds...";
        }
    }
}
