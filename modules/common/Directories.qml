pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common.functions
import QtCore
import QtQuick
import Quickshell

Singleton {
    // XDG Dirs, with "file://"
    readonly property string home: StandardPaths.standardLocations(StandardPaths.HomeLocation)[0]
    readonly property string config: StandardPaths.standardLocations(StandardPaths.ConfigLocation)[0]
    readonly property string state: StandardPaths.standardLocations(StandardPaths.StateLocation)[0]
    readonly property string cache: StandardPaths.standardLocations(StandardPaths.CacheLocation)[0]
    readonly property string genericCache: StandardPaths.standardLocations(StandardPaths.GenericCacheLocation)[0]
    readonly property string documents: StandardPaths.standardLocations(StandardPaths.DocumentsLocation)[0]
    readonly property string downloads: StandardPaths.standardLocations(StandardPaths.DownloadLocation)[0]
    readonly property string pictures: StandardPaths.standardLocations(StandardPaths.PicturesLocation)[0]
    readonly property string music: StandardPaths.standardLocations(StandardPaths.MusicLocation)[0]
    readonly property string videos: StandardPaths.standardLocations(StandardPaths.MoviesLocation)[0]

    // Shell paths (relative to hamr installation)
    property string assetsPath: Quickshell.shellPath("assets")
    property string scriptPath: Quickshell.shellPath("scripts")
    
    // Hamr config folder: ~/.config/hamr/
    property string hamrConfig: FileUtils.trimFileProtocol(`${Directories.config}/hamr`)
    
    // External theme integration (e.g., end-4's illogical-impulse generates colors here)
    property string externalThemeState: FileUtils.trimFileProtocol(`${Directories.state}/user`)
    property string generatedMaterialThemePath: FileUtils.trimFileProtocol(`${Directories.externalThemeState}/generated/colors.json`)
    
    // Hamr data paths
    property string userPlugins: FileUtils.trimFileProtocol(`${Directories.hamrConfig}/plugins`)
    property string quicklinksConfig: FileUtils.trimFileProtocol(`${Directories.hamrConfig}/quicklinks.json`)
    property string searchHistory: FileUtils.trimFileProtocol(`${Directories.hamrConfig}/search-history.json`)
    property string shellConfigPath: FileUtils.trimFileProtocol(`${Directories.hamrConfig}/config.json`)
    property string favicons: FileUtils.trimFileProtocol(`${Directories.cache}/favicons`)
    
    // Default wallpaper directory
    property string defaultWallpaperDir: FileUtils.trimFileProtocol(`${Directories.pictures}/Wallpapers`)
    
    // Initialize directories on startup
    Component.onCompleted: {
        Quickshell.execDetached(["mkdir", "-p", hamrConfig])
        Quickshell.execDetached(["mkdir", "-p", userPlugins])
        Quickshell.execDetached(["mkdir", "-p", favicons])
    }
}
