pragma Singleton
pragma ComponentBehavior: Bound

import qs.modules.common
import qs.modules.common.models
import qs.modules.common.functions
import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io

/**
 * Provides folder browsing with image filtering, thumbnail generation, and OCR indexing.
 * Used by the ImageBrowser component for workflows.
 */
Singleton {
    id: root

    property string thumbgenScriptPath: `${FileUtils.trimFileProtocol(Directories.scriptPath)}/thumbnails/thumbgen-venv.sh`
    property string generateThumbnailsMagickScriptPath: `${FileUtils.trimFileProtocol(Directories.scriptPath)}/thumbnails/generate-thumbnails-magick.sh`
    property string ocrIndexScriptPath: `${FileUtils.trimFileProtocol(Directories.scriptPath)}/ocr/ocr-index-venv.sh`
    property alias directory: folderModel.folder
    readonly property string effectiveDirectory: FileUtils.trimFileProtocol(folderModel.folder.toString())
    property url defaultFolder: Qt.resolvedUrl(Directories.defaultWallpaperDir)
    property alias folderModel: folderModel
    property string searchQuery: ""
    readonly property list<string> extensions: [
        "jpg", "jpeg", "png", "webp", "avif", "bmp", "svg"
    ]
    property list<string> files: [] // List of absolute file paths (without file://)
    readonly property bool thumbnailGenerationRunning: thumbgenProc.running
    property real thumbnailGenerationProgress: 0
    
    // OCR indexing
    readonly property bool ocrIndexingRunning: ocrProc.running
    property real ocrIndexingProgress: 0
    property var ocrIndex: ({})  // Map of filePath -> OCR text
    property bool ocrEnabled: false  // Set to true to enable OCR indexing for current directory

    signal changed()
    signal thumbnailGenerated(directory: string)
    signal thumbnailGeneratedFile(filePath: string)
    signal ocrIndexed(directory: string)
    signal ocrIndexedFile(filePath: string, text: string)

    function load() {} // For forcing initialization

    // Directory navigation
    Process {
        id: validateDirProc
        property string nicePath: ""
        function setDirectoryIfValid(path) {
            validateDirProc.nicePath = FileUtils.trimFileProtocol(path).replace(/\/+$/, "")
            if (/^\/*$/.test(validateDirProc.nicePath)) validateDirProc.nicePath = "/";
            validateDirProc.exec([
                "bash", "-c",
                `if [ -d "${validateDirProc.nicePath}" ]; then echo dir; elif [ -f "${validateDirProc.nicePath}" ]; then echo file; else echo invalid; fi`
            ])
        }
        stdout: StdioCollector {
            onStreamFinished: {
                root.directory = Qt.resolvedUrl(validateDirProc.nicePath)
                const result = text.trim()
                if (result === "dir") {
                    // Already set above
                } else if (result === "file") {
                    root.directory = Qt.resolvedUrl(FileUtils.parentDirectory(validateDirProc.nicePath))
                } else {
                    // Ignore invalid paths
                }
            }
        }
    }

    function setDirectory(path) {
        validateDirProc.setDirectoryIfValid(path)
    }

    function navigateUp() {
        folderModel.navigateUp()
    }

    function navigateBack() {
        folderModel.navigateBack()
    }

    function navigateForward() {
        folderModel.navigateForward()
    }

    // Folder model - base model without search filtering (we filter ourselves for OCR support)
    FolderListModelWithHistory {
        id: folderModel
        folder: Qt.resolvedUrl(root.defaultFolder)
        caseSensitive: false
        nameFilters: root.extensions.map(ext => `*.${ext}`)
        showDirs: true
        showDotAndDotDot: false
        showOnlyReadable: true
        sortField: FolderListModel.Time
        sortReversed: false
        onCountChanged: {
            root.files = []
            for (let i = 0; i < folderModel.count; i++) {
                const path = folderModel.get(i, "filePath") || FileUtils.trimFileProtocol(folderModel.get(i, "fileURL"))
                if (path && path.length) root.files.push(path)
            }
            // Rebuild filtered model when source changes
            root.rebuildFilteredModel()
        }
    }
    
    // Filtered model that supports both filename and OCR text search
    ListModel {
        id: filteredFolderModel
    }
    property alias filteredModel: filteredFolderModel
    
    // Rebuild filtered model based on searchQuery and OCR index
    function rebuildFilteredModel() {
        filteredFolderModel.clear();
        
        const query = root.searchQuery.trim().toLowerCase();
        const queryParts = query ? query.split(/\s+/).filter(s => s.length > 0) : [];
        
        for (let i = 0; i < folderModel.count; i++) {
            const filePath = folderModel.get(i, "filePath") || FileUtils.trimFileProtocol(folderModel.get(i, "fileURL"));
            const fileName = folderModel.get(i, "fileName");
            const fileIsDir = folderModel.get(i, "fileIsDir");
            
            // Directories always pass (user can navigate)
            if (fileIsDir) {
                filteredFolderModel.append({
                    filePath: filePath,
                    fileName: fileName,
                    fileIsDir: fileIsDir
                });
                continue;
            }
            
            // No query = show all files
            if (queryParts.length === 0) {
                filteredFolderModel.append({
                    filePath: filePath,
                    fileName: fileName,
                    fileIsDir: fileIsDir
                });
                continue;
            }
            
            // Check filename match
            const fileNameLower = fileName.toLowerCase();
            const fileNameMatches = queryParts.every(part => fileNameLower.includes(part));
            
            // Check OCR text match (if OCR is enabled and indexed)
            const ocrText = root.ocrIndex[filePath] ?? "";
            const ocrTextLower = ocrText.toLowerCase();
            const ocrMatches = ocrText && queryParts.every(part => ocrTextLower.includes(part));
            
            if (fileNameMatches || ocrMatches) {
                filteredFolderModel.append({
                    filePath: filePath,
                    fileName: fileName,
                    fileIsDir: fileIsDir
                });
            }
        }
    }
    
    // Rebuild filter when search query changes
    onSearchQueryChanged: rebuildFilteredModel()
    
    // Rebuild filter when OCR index updates
    onOcrIndexChanged: {
        if (searchQuery.trim().length > 0) {
            rebuildFilteredModel()
        }
    }

    // Thumbnail generation
    function generateThumbnail(size: string) {
        if (!["normal", "large", "x-large", "xx-large"].includes(size)) throw new Error("Invalid thumbnail size");
        thumbgenProc.directory = root.directory
        thumbgenProc.running = false
        thumbgenProc.command = [
            "bash", "-c",
            `${thumbgenScriptPath} --size ${size} --machine_progress -d ${FileUtils.trimFileProtocol(root.directory)} || ${generateThumbnailsMagickScriptPath} --size ${size} -d ${FileUtils.trimFileProtocol(root.directory)}`,
        ]
        root.thumbnailGenerationProgress = 0
        thumbgenProc.running = true
    }

    Process {
        id: thumbgenProc
        property string directory
        stdout: SplitParser {
            onRead: data => {
                let match = data.match(/PROGRESS (\d+)\/(\d+)/)
                if (match) {
                    const completed = parseInt(match[1])
                    const total = parseInt(match[2])
                    root.thumbnailGenerationProgress = completed / total
                }
                match = data.match(/FILE (.+)/)
                if (match) {
                    const filePath = match[1]
                    root.thumbnailGeneratedFile(filePath)
                }
            }
        }
        onExited: (exitCode, exitStatus) => {
            root.thumbnailGenerated(thumbgenProc.directory)
        }
    }
    
    // ==================== OCR INDEXING ====================
    
    // Start OCR indexing for current directory
    function generateOcrIndex() {
        if (ocrProc.running) return;
        
        ocrProc.directory = root.effectiveDirectory;
        root.ocrIndex = {};
        root.ocrIndexingProgress = 0;
        ocrProc.command = [
            "bash", "-c",
            `${ocrIndexScriptPath} -d "${root.effectiveDirectory}" --machine_progress`
        ];
        ocrProc.running = true;
    }
    
    // Check if a file's OCR text matches the search query
    function fileMatchesOcrQuery(filePath: string, query: string): bool {
        if (!query || query.trim() === "") return true;
        const ocrText = root.ocrIndex[filePath] ?? "";
        if (!ocrText) return false;
        
        const queryLower = query.toLowerCase();
        const textLower = ocrText.toLowerCase();
        const queryParts = queryLower.split(/\s+/).filter(s => s.length > 0);
        
        return queryParts.every(part => textLower.includes(part));
    }
    
    // Get OCR text for a file
    function getOcrText(filePath: string): string {
        return root.ocrIndex[filePath] ?? "";
    }
    
    Process {
        id: ocrProc
        property string directory
        stdout: SplitParser {
            onRead: data => {
                // Parse PROGRESS lines
                let match = data.match(/PROGRESS (\d+)\/(\d+)/)
                if (match) {
                    const completed = parseInt(match[1])
                    const total = parseInt(match[2])
                    root.ocrIndexingProgress = total > 0 ? completed / total : 0
                }
                
                // Parse OCR lines: OCR /path/to/file|extracted text
                match = data.match(/^OCR (.+?)\|(.*)$/)
                if (match) {
                    const filePath = match[1]
                    // Unescape newlines
                    const text = match[2].replace(/\\n/g, "\n").replace(/\\\\/g, "\\")
                    
                    // Update index (use Object.assign for QML JS compatibility)
                    const newIndex = Object.assign({}, root.ocrIndex)
                    newIndex[filePath] = text
                    root.ocrIndex = newIndex
                    
                    root.ocrIndexedFile(filePath, text)
                }
            }
        }
        onExited: (exitCode, exitStatus) => {
            root.ocrIndexed(ocrProc.directory)
        }
    }
}
