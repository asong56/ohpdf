#!/usr/bin/env bash
# Install OhPDF right-click (Quick Action) services on macOS.
# Run once after placing the ohpdf binary in /usr/local/bin/.
#
# Creates Automator Quick Actions in ~/Library/Services/ that appear
# when right-clicking PDF files in Finder.
set -euo pipefail

OHPDF="$(command -v ohpdf 2>/dev/null || echo /usr/local/bin/ohpdf)"
SERVICES_DIR="$HOME/Library/Services"
mkdir -p "$SERVICES_DIR"

make_service() {
  local name="$1"   # Display name, e.g. "OhPDF - Compress"
  local action="$2" # ohpdf subcommand
  local extra="$3"  # extra CLI args (may be empty)
  local bundle="$SERVICES_DIR/${name}.workflow"
  local contents="$bundle/Contents"
  mkdir -p "$contents"

  cat > "$contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSServices</key>
  <array>
    <dict>
      <key>NSMenuItem</key>
      <dict><key>default</key><string>OhPDF — ${name#OhPDF - }</string></dict>
      <key>NSMessage</key><string>runWorkflowAsService</string>
      <key>NSRequiredContext</key>
      <dict>
        <key>NSApplicationIdentifier</key><string>com.apple.finder</string>
      </dict>
      <key>NSSendFileTypes</key>
      <array><string>com.adobe.pdf</string></array>
    </dict>
  </array>
</dict>
</plist>
PLIST

  cat > "$contents/document.wflow" << WFLOW
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>AMApplicationBuild</key><string>523</string>
  <key>AMApplicationVersion</key><string>2.10</string>
  <key>AMDocumentVersion</key><string>2</string>
  <key>actions</key>
  <array>
    <dict>
      <key>action</key>
      <dict>
        <key>AMAccepts</key>
        <dict><key>Container</key><string>List</string><key>Optional</key><true/><key>Types</key><array><string>com.apple.applescript.file</string></array></dict>
        <key>AMActionVersion</key><string>2.0.3</string>
        <key>AMApplication</key><array><string>Automator</string></array>
        <key>AMParameterProperties</key><dict><key>source</key><dict/></dict>
        <key>AMProvides</key><dict><key>Container</key><string>List</string><key>Types</key><array><string>com.apple.applescript.file</string></array></dict>
        <key>ActionBundlePath</key><string>/System/Library/Automator/Run Shell Script.action</string>
        <key>ActionName</key><string>Run Shell Script</string>
        <key>ActionParameters</key>
        <dict>
          <key>COMMAND_STRING</key>
          <string>for f in "\$@"; do
  out="\$(dirname "\$f")/\$(basename "\$f" .pdf)_${action}.pdf"
  "${OHPDF}" ${action} "\$f" "\$out" ${extra} &amp;
done
wait</string>
          <key>CheckedForUserDefaultShell</key><true/>
          <key>inputMethod</key><integer>1</integer>
          <key>shell</key><string>/bin/bash</string>
          <key>source</key><string></string>
        </dict>
        <key>BundleIdentifier</key><string>com.apple.automator.shellscript</string>
        <key>CFBundleVersion</key><string>2.0.3</string>
        <key>CanShowSelectedItemsWhenRun</key><false/>
        <key>CanShowWhenRun</key><true/>
        <key>Category</key><array><string>AMCategoryUtilities</string></array>
        <key>Class Name</key><string>RunShellScriptAction</string>
        <key>InputUUID</key><string>$(uuidgen)</string>
        <key>Keywords</key><array><string>Shell</string><string>Script</string><string>Command</string><string>Run</string><string>Unix</string></array>
        <key>OutputUUID</key><string>$(uuidgen)</string>
        <key>UUID</key><string>$(uuidgen)</string>
        <key>UnlocalizedApplications</key><array><string>Automator</string></array>
        <key>arguments</key><dict/>
        <key>conversionLabel</key><integer>0</integer>
        <key>isViewVisible</key><true/>
        <key>location</key><string>309.5:153.0</string>
        <key>nibPath</key><string>/System/Library/Automator/Run Shell Script.action/Contents/Resources/Base.lproj/main.xib</string>
      </dict>
      <key>isViewVisible</key><true/>
    </dict>
  </array>
  <key>connectors</key><dict/>
  <key>workflowMetaData</key>
  <dict>
    <key>serviceInputTypeIdentifier</key><string>com.apple.Automator.fileSystemObject.pdf</string>
    <key>serviceOutputTypeIdentifier</key><string>com.apple.Automator.nothing</string>
    <key>serviceProcessesInput</key><integer>0</integer>
    <key>workflowTypeIdentifier</key><string>com.apple.Automator.servicesMenu</string>
  </dict>
</dict>
</plist>
WFLOW
  echo "  ✓ Created: $bundle"
}

echo "Installing OhPDF Quick Actions…"
make_service "OhPDF - Compress"  "compress"  ""
make_service "OhPDF - Encrypt"   "encrypt"   "-p \"\$(osascript -e 'text returned of (display dialog \"Password for encryption:\" default answer \"\" with hidden answer)')\""
make_service "OhPDF - Watermark" "watermark" "-t Confidential"
make_service "OhPDF - To Images" "to-images" ""

# Reload services
/System/Library/CoreServices/pbs -flush 2>/dev/null || true
echo ""
echo "Done! Right-click a PDF in Finder to see the OhPDF menu."
echo "Note: You may need to log out and back in for services to appear."
