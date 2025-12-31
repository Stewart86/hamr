/**
 * PluginForm - Displays a form with multiple input fields from a plugin handler
 * 
 * Used for plugins that need multiple inputs (e.g., notes with title+content,
 * settings, multi-field submissions) rather than sequential single-line prompts.
 * 
 * Field types supported:
 *   - text: Single-line text input
 *   - textarea: Multi-line text input
 *   - select: Dropdown selection
 *   - checkbox: Boolean toggle
 *   - password: Hidden text input
 */
import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell

Rectangle {
    id: root
    
     // Form data from PluginRunner.pluginForm
     // { title: string, submitLabel: string, cancelLabel: string, fields: [...] }
    property var form: null
    
    // Emitted when form is submitted with all field values
    signal submitted(var formData)
    
    // Emitted when form is cancelled
    signal cancelled()
    
    property string title: form?.title ?? ""
    property string submitLabel: form?.submitLabel ?? "Submit"
    property string cancelLabel: form?.cancelLabel ?? "Cancel"
    property var fields: form?.fields ?? []
    
    // Store field values separately to persist across visibility changes
    // Key: field id, Value: current value
    property var fieldValues: ({})
    
    // Track which form we're showing to reset values when form changes
    property string currentFormId: ""
    
    visible: form !== null
    
    // Reset field values when a new form is shown
    onFormChanged: {
        if (form) {
            // Always reset password fields for security, and reset all fields on new form
            let newFormId = form.id ?? ((form.title ?? "") + "_" + (form.fields?.length ?? 0));
            
            // Reset stored values and initialize from defaults
            fieldValues = {};
            if (form.fields) {
                for (let field of form.fields) {
                    if (field.id) {
                        fieldValues[field.id] = field.default ?? "";
                    }
                }
            }
            currentFormId = newFormId;
        }
    }
    
    Layout.fillWidth: true
    implicitHeight: Math.min(500, formColumn.implicitHeight + 32)
    
    color: "transparent"
    
    // Collect form data from all fields
    function collectFormData() {
        // Sync current field values to storage before collecting
        syncFieldValues();
        return Object.assign({}, fieldValues);
    }
    
    // Sync current input values to the fieldValues storage
    function syncFieldValues() {
        for (let i = 0; i < fieldsRepeater.count; i++) {
            let item = fieldsRepeater.itemAt(i);
            if (item && item.fieldId) {
                fieldValues[item.fieldId] = item.fieldValue;
            }
        }
    }
    
    // Update a specific field value (called by field components on change)
    function updateFieldValue(fieldId, value) {
        fieldValues[fieldId] = value;
    }
    
    // Get stored value for a field
    function getFieldValue(fieldId, defaultValue) {
        if (fieldId in fieldValues) {
            return fieldValues[fieldId];
        }
        return defaultValue ?? "";
    }
    
    // Validate required fields
    function validateForm() {
        for (let i = 0; i < fieldsRepeater.count; i++) {
            let item = fieldsRepeater.itemAt(i);
            if (item && item.required && !item.fieldValue) {
                return false;
            }
        }
        return true;
    }
    
    // Focus first field
    function focusFirstField() {
        if (fieldsRepeater.count > 0) {
            let first = fieldsRepeater.itemAt(0);
            if (first && first.focusField) {
                first.focusField();
            }
        }
    }
    
    ColumnLayout {
        id: formColumn
        anchors {
            fill: parent
            margins: 16
        }
        spacing: 12
        
        // Title
        StyledText {
            Layout.fillWidth: true
            visible: root.title !== ""
            text: root.title
            font.pixelSize: Appearance.font.pixelSize.normal
            font.weight: Font.DemiBold
            color: Appearance.m3colors.m3onSurface
            wrapMode: Text.Wrap
        }
        
        // Separator after title
        Rectangle {
            Layout.fillWidth: true
            visible: root.title !== ""
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Form fields container with scroll
        ScrollView {
            id: fieldsScrollView
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredHeight: Math.min(350, fieldsColumn.implicitHeight)
            
            clip: true
            
            ScrollBar.vertical: StyledScrollBar {
                policy: ScrollBar.AsNeeded
            }
            ScrollBar.horizontal: ScrollBar {
                policy: ScrollBar.AlwaysOff
            }
            
            ColumnLayout {
                id: fieldsColumn
                width: fieldsScrollView.availableWidth
                spacing: 16
                
                Repeater {
                    id: fieldsRepeater
                    model: root.fields
                    
                    delegate: Loader {
                        id: fieldLoader
                        Layout.fillWidth: true
                        
                        required property var modelData
                        required property int index
                        
                        // Expose field properties for parent access
                        property string fieldId: modelData.id ?? ""
                        property var fieldValue: item?.value ?? ""
                        property bool required: modelData.required ?? false
                        
                        function focusField() {
                            if (item && item.focusInput) {
                                item.focusInput();
                            }
                        }
                        
                        sourceComponent: {
                            switch (modelData.type) {
                                case "textarea": return textAreaField;
                                case "select": return selectField;
                                case "checkbox": return checkboxField;
                                case "password": return passwordField;
                                default: return textField;
                            }
                        }
                        
                        onLoaded: {
                            if (item) {
                                item.fieldData = modelData;
                            }
                        }
                    }
                }
            }
        }
        
        // Separator before buttons
        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: Appearance.colors.colOutlineVariant
        }
        
        // Button row
        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            
            Item { Layout.fillWidth: true }
            
            // Cancel button with Esc hint
            RippleButton {
                implicitHeight: 36
                implicitWidth: cancelRow.implicitWidth + 24
                buttonRadius: Appearance.rounding.small
                
                colBackground: Appearance.colors.colSurfaceContainerHigh
                colBackgroundHover: Appearance.colors.colSurfaceContainerHighest
                colRipple: Appearance.colors.colOutlineVariant
                
                RowLayout {
                    id: cancelRow
                    anchors.centerIn: parent
                    spacing: 6
                    
                    StyledText {
                        text: root.cancelLabel
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: Appearance.m3colors.m3onSurface
                    }
                    
                    Kbd {
                        keys: "Bksp"
                    }
                }
                
                onClicked: root.cancelled()
            }
            
            // Submit button with Ctrl+Enter hint
            RippleButton {
                implicitHeight: 36
                implicitWidth: submitRow.implicitWidth + 24
                buttonRadius: Appearance.rounding.small
                
                colBackground: Appearance.colors.colPrimary
                colBackgroundHover: Appearance.colors.colPrimaryHover
                colRipple: Appearance.colors.colPrimaryActive
                
                RowLayout {
                    id: submitRow
                    anchors.centerIn: parent
                    spacing: 6
                    
                    StyledText {
                        text: root.submitLabel
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: Appearance.m3colors.m3onPrimary
                    }
                    
                    Kbd {
                        keys: "^Enter"
                        color: Appearance.colors.colPrimaryHover
                        border.color: Appearance.m3colors.m3onPrimary
                        textColor: Appearance.m3colors.m3onPrimary
                    }
                }
                
                onClicked: {
                    if (root.validateForm()) {
                        root.submitted(root.collectFormData());
                    }
                }
            }
        }
    }
    
    // ==================== FIELD COMPONENTS ====================
    
    // Text field (single line)
    Component {
        id: textField
        
        ColumnLayout {
            id: textFieldRoot
            property var fieldData: ({})
            property string value: textInput.text
            
            function focusInput() {
                textInput.forceActiveFocus();
            }
            
            spacing: 4
            
            StyledText {
                visible: fieldData.label ?? false
                text: (fieldData.label ?? "") + (fieldData.required ? " *" : "")
                font.pixelSize: Appearance.font.pixelSize.small
                font.weight: Font.Medium
                color: Appearance.m3colors.m3onSurface
            }
            
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: textInput.activeFocus ? 2 : 1
                border.color: textInput.activeFocus ? Appearance.colors.colPrimary : Appearance.colors.colOutlineVariant
                
                TextField {
                    id: textInput
                    anchors {
                        fill: parent
                        margins: 2
                    }
                    
                    // Initialize from stored value or default
                    text: root.getFieldValue(fieldData.id, fieldData.default ?? "")
                    placeholderText: fieldData.placeholder ?? ""
                    
                    font.family: Appearance.font.family.main
                    font.pixelSize: Appearance.font.pixelSize.small
                    color: Appearance.m3colors.m3onSurface
                    placeholderTextColor: Appearance.colors.colSubtext
                    selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                    selectionColor: Appearance.colors.colSecondaryContainer
                    
                    background: null
                    
                    // Persist value on change
                    onTextChanged: root.updateFieldValue(fieldData.id, text)
                    
                    Keys.onPressed: event => {
                        // Escape to cancel form
                        if (event.key === Qt.Key_Escape) {
                            root.cancelled();
                            event.accepted = true;
                            return;
                        }
                        // Ctrl+Enter to submit
                        if ((event.modifiers & Qt.ControlModifier) && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                            if (root.validateForm()) {
                                root.submitted(root.collectFormData());
                            }
                            event.accepted = true;
                            return;
                        }
                    }
                }
            }
            
            StyledText {
                visible: fieldData.hint ?? false
                text: fieldData.hint ?? ""
                font.pixelSize: Appearance.font.pixelSize.smaller
                color: Appearance.colors.colSubtext
            }
        }
    }
    
    // Password field
    Component {
        id: passwordField
        
        ColumnLayout {
            id: passwordFieldRoot
            property var fieldData: ({})
            property string value: passwordInput.text
            
            function focusInput() {
                passwordInput.forceActiveFocus();
            }
            
            spacing: 4
            
            StyledText {
                visible: fieldData.label ?? false
                text: (fieldData.label ?? "") + (fieldData.required ? " *" : "")
                font.pixelSize: Appearance.font.pixelSize.small
                font.weight: Font.Medium
                color: Appearance.m3colors.m3onSurface
            }
            
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 40
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: passwordInput.activeFocus ? 2 : 1
                border.color: passwordInput.activeFocus ? Appearance.colors.colPrimary : Appearance.colors.colOutlineVariant
                
                TextField {
                    id: passwordInput
                    anchors {
                        fill: parent
                        margins: 2
                    }
                    
                    // Initialize from stored value or default
                    text: root.getFieldValue(fieldData.id, fieldData.default ?? "")
                    placeholderText: fieldData.placeholder ?? ""
                    echoMode: TextInput.Password
                    
                    font.family: Appearance.font.family.main
                    font.pixelSize: Appearance.font.pixelSize.small
                    color: Appearance.m3colors.m3onSurface
                    placeholderTextColor: Appearance.colors.colSubtext
                    selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                    selectionColor: Appearance.colors.colSecondaryContainer
                    
                    background: null
                    
                    // Persist value on change
                    onTextChanged: root.updateFieldValue(fieldData.id, text)
                    
                    Keys.onPressed: event => {
                        if (event.key === Qt.Key_Escape) {
                            root.cancelled();
                            event.accepted = true;
                            return;
                        }
                        if ((event.modifiers & Qt.ControlModifier) && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                            if (root.validateForm()) {
                                root.submitted(root.collectFormData());
                            }
                            event.accepted = true;
                            return;
                        }
                    }
                }
            }
        }
    }
    
    // TextArea field (multi-line)
    Component {
        id: textAreaField
        
        ColumnLayout {
            id: textAreaFieldRoot
            property var fieldData: ({})
            property string value: textAreaInput.text
            
            function focusInput() {
                textAreaInput.forceActiveFocus();
            }
            
            spacing: 4
            
            StyledText {
                visible: fieldData.label ?? false
                text: (fieldData.label ?? "") + (fieldData.required ? " *" : "")
                font.pixelSize: Appearance.font.pixelSize.small
                font.weight: Font.Medium
                color: Appearance.m3colors.m3onSurface
            }
            
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Math.max(100, (fieldData.rows ?? 4) * 24)
                radius: Appearance.rounding.small
                color: Appearance.colors.colSurfaceContainerHigh
                border.width: textAreaInput.activeFocus ? 2 : 1
                border.color: textAreaInput.activeFocus ? Appearance.colors.colPrimary : Appearance.colors.colOutlineVariant
                
                ScrollView {
                    anchors {
                        fill: parent
                        margins: 4
                    }
                    
                    clip: true
                    
                    ScrollBar.vertical: StyledScrollBar {
                        policy: ScrollBar.AsNeeded
                    }
                    ScrollBar.horizontal: ScrollBar {
                        policy: ScrollBar.AlwaysOff
                    }
                    
                    TextArea {
                        id: textAreaInput
                        width: parent.availableWidth
                        
                        // Initialize from stored value or default
                        text: root.getFieldValue(fieldData.id, fieldData.default ?? "")
                        placeholderText: fieldData.placeholder ?? ""
                        wrapMode: TextEdit.Wrap
                        
                        font.family: Appearance.font.family.main
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: Appearance.m3colors.m3onSurface
                        placeholderTextColor: Appearance.colors.colSubtext
                        selectedTextColor: Appearance.m3colors.m3onSecondaryContainer
                        selectionColor: Appearance.colors.colSecondaryContainer
                        
                        background: null
                        
                        // Persist value on change
                        onTextChanged: root.updateFieldValue(fieldData.id, text)
                        
                        Keys.onPressed: event => {
                            // Escape to cancel form
                            if (event.key === Qt.Key_Escape) {
                                root.cancelled();
                                event.accepted = true;
                                return;
                            }
                            // Ctrl+Enter to submit form
                            if ((event.modifiers & Qt.ControlModifier) && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                                if (root.validateForm()) {
                                    root.submitted(root.collectFormData());
                                }
                                event.accepted = true;
                                return;
                            }
                        }
                    }
                }
            }
            
            StyledText {
                visible: fieldData.hint ?? false
                text: fieldData.hint ?? ""
                font.pixelSize: Appearance.font.pixelSize.smaller
                color: Appearance.colors.colSubtext
            }
        }
    }
    
    // Select field (dropdown)
    Component {
        id: selectField
        
        ColumnLayout {
            id: selectFieldRoot
            property var fieldData: ({})
            property var options: fieldData.options ?? []
            property string value: {
                if (selectCombo.currentIndex >= 0 && selectCombo.currentIndex < options.length) {
                    const o = options[selectCombo.currentIndex];
                    return o.value ?? o.id ?? o.name ?? "";
                }
                return "";
            }
            
            function focusInput() {
                selectCombo.forceActiveFocus();
            }
            
            function getOptionValue(o) {
                return o.value ?? o.id ?? o.name ?? "";
            }
            
            function getOptionLabel(o) {
                return o.label ?? o.name ?? o.id ?? o.value ?? "";
            }
            
            // Get initial index from stored value or default
            function getInitialIndex() {
                let storedVal = root.getFieldValue(fieldData.id, fieldData.default ?? "");
                if (!storedVal) return 0;
                let idx = options.findIndex(o => getOptionValue(o) === storedVal);
                return idx >= 0 ? idx : 0;
            }
            
            spacing: 4
            
            StyledText {
                visible: fieldData.label ?? false
                text: (fieldData.label ?? "") + (fieldData.required ? " *" : "")
                font.pixelSize: Appearance.font.pixelSize.small
                font.weight: Font.Medium
                color: Appearance.m3colors.m3onSurface
            }
            
            ComboBox {
                id: selectCombo
                Layout.fillWidth: true
                
                model: selectFieldRoot.options.map(o => selectFieldRoot.getOptionLabel(o))
                
                currentIndex: selectFieldRoot.getInitialIndex()
                
                // Persist value on change
                onCurrentIndexChanged: {
                    if (currentIndex >= 0 && currentIndex < selectFieldRoot.options.length) {
                        let val = selectFieldRoot.getOptionValue(selectFieldRoot.options[currentIndex]);
                        root.updateFieldValue(fieldData.id, val);
                    }
                }
                
                font.family: Appearance.font.family.main
                font.pixelSize: Appearance.font.pixelSize.small
                
                background: Rectangle {
                    implicitHeight: 40
                    radius: Appearance.rounding.small
                    color: Appearance.colors.colSurfaceContainerHigh
                    border.width: selectCombo.activeFocus ? 2 : 1
                    border.color: selectCombo.activeFocus ? Appearance.colors.colPrimary : Appearance.colors.colOutlineVariant
                }
                
                contentItem: Text {
                    leftPadding: 12
                    text: selectCombo.displayText
                    font: selectCombo.font
                    color: Appearance.m3colors.m3onSurface
                    verticalAlignment: Text.AlignVCenter
                }
                
                indicator: MaterialSymbol {
                    x: selectCombo.width - width - 8
                    y: (selectCombo.height - height) / 2
                    text: "expand_more"
                    iconSize: Appearance.font.pixelSize.normal
                    color: Appearance.m3colors.m3onSurface
                }
                
                delegate: ItemDelegate {
                    width: selectCombo.width
                    contentItem: Text {
                        text: modelData
                        color: Appearance.m3colors.m3onSurface
                        font: selectCombo.font
                        verticalAlignment: Text.AlignVCenter
                    }
                    background: Rectangle {
                        color: highlighted ? Appearance.colors.colSurfaceContainerHighest : "transparent"
                    }
                    highlighted: selectCombo.highlightedIndex === index
                }
                
                popup: Popup {
                    y: selectCombo.height
                    width: selectCombo.width
                    implicitHeight: contentItem.implicitHeight + 2
                    padding: 1
                    
                    contentItem: ListView {
                        clip: true
                        implicitHeight: contentHeight
                        model: selectCombo.popup.visible ? selectCombo.delegateModel : null
                        currentIndex: selectCombo.highlightedIndex
                    }
                    
                    background: Rectangle {
                        color: Appearance.colors.colSurfaceContainerHigh
                        border.color: Appearance.colors.colOutlineVariant
                        border.width: 1
                        radius: Appearance.rounding.small
                    }
                }
                
                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Escape) {
                        root.cancelled();
                        event.accepted = true;
                        return;
                    }
                    if ((event.modifiers & Qt.ControlModifier) && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                        if (root.validateForm()) {
                            root.submitted(root.collectFormData());
                        }
                        event.accepted = true;
                        return;
                    }
                }
            }
        }
    }
    
    // Checkbox field
    Component {
        id: checkboxField
        
        RowLayout {
            id: checkboxFieldRoot
            property var fieldData: ({})
            property bool value: checkboxInput.checked
            
            function focusInput() {
                checkboxInput.forceActiveFocus();
            }
            
            // Get initial checked state from stored value or default
            function getInitialChecked() {
                let stored = root.getFieldValue(fieldData.id, null);
                if (stored !== null && stored !== "") {
                    return stored === true || stored === "true";
                }
                return fieldData.default ?? false;
            }
            
            spacing: 8
            
            CheckBox {
                id: checkboxInput
                checked: checkboxFieldRoot.getInitialChecked()
                
                // Persist value on change
                onCheckedChanged: root.updateFieldValue(fieldData.id, checked)
                
                indicator: Rectangle {
                    implicitWidth: 20
                    implicitHeight: 20
                    x: checkboxInput.leftPadding
                    y: (checkboxInput.height - height) / 2
                    radius: Appearance.rounding.verysmall
                    color: checkboxInput.checked ? Appearance.colors.colPrimary : Appearance.colors.colSurfaceContainerHigh
                    border.width: checkboxInput.checked ? 0 : 1
                    border.color: Appearance.colors.colOutlineVariant
                    
                    MaterialSymbol {
                        anchors.centerIn: parent
                        visible: checkboxInput.checked
                        text: "check"
                        iconSize: 16
                        color: Appearance.m3colors.m3onPrimary
                    }
                }
                
                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Escape) {
                        root.cancelled();
                        event.accepted = true;
                        return;
                    }
                    if ((event.modifiers & Qt.ControlModifier) && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                        if (root.validateForm()) {
                            root.submitted(root.collectFormData());
                        }
                        event.accepted = true;
                        return;
                    }
                }
            }
            
            StyledText {
                text: fieldData.label ?? ""
                font.pixelSize: Appearance.font.pixelSize.small
                color: Appearance.m3colors.m3onSurface
                
                MouseArea {
                    anchors.fill: parent
                    onClicked: checkboxInput.checked = !checkboxInput.checked
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }
    }
    
    // Auto-focus first field when form becomes visible
    onVisibleChanged: {
        if (visible) {
            Qt.callLater(focusFirstField);
        }
    }
    
}
