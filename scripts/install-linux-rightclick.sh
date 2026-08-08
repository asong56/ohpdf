#!/usr/bin/env bash
# Install OhPDF right-click actions for Nautilus (GNOME Files) and Nemo (Cinnamon).
# Run once after placing the ohpdf binary in /usr/local/bin/.
set -euo pipefail

OHPDF="$(command -v ohpdf 2>/dev/null || echo /usr/local/bin/ohpdf)"

# ── Nautilus scripts ──────────────────────────────────────────────────────────
NAUTILUS_DIR="$HOME/.local/share/nautilus/scripts"
mkdir -p "$NAUTILUS_DIR"

make_nautilus() {
  local name="$1"
  local body="$2"
  local file="$NAUTILUS_DIR/OhPDF - ${name}"
  cat > "$file" << SCRIPT
#!/usr/bin/env bash
# OhPDF Nautilus script — ${name}
IFS=$'\n'
${body}
SCRIPT
  chmod +x "$file"
  echo "  ✓ Nautilus: OhPDF - ${name}"
}

make_nautilus "Compress" \
'for f in $NAUTILUS_SCRIPT_SELECTED_FILE_PATHS; do
  [[ "$f" == *.pdf ]] || continue
  out="${f%.pdf}_compressed.pdf"
  '"$OHPDF"' compress "$f" "$out" &
done; wait; notify-send "OhPDF" "Compression complete."'

make_nautilus "Merge Selected" \
'files=($NAUTILUS_SCRIPT_SELECTED_FILE_PATHS)
pdfs=()
for f in "${files[@]}"; do [[ "$f" == *.pdf ]] && pdfs+=("$f"); done
[[ ${#pdfs[@]} -ge 2 ]] || { notify-send "OhPDF" "Select 2+ PDFs to merge."; exit 1; }
out="$(dirname "${pdfs[0]}")/merged.pdf"
'"$OHPDF"' merge "${pdfs[@]}" -o "$out"
notify-send "OhPDF" "Merged → $out"'

make_nautilus "To Images" \
'for f in $NAUTILUS_SCRIPT_SELECTED_FILE_PATHS; do
  [[ "$f" == *.pdf ]] || continue
  '"$OHPDF"' to-images "$f" &
done; wait; notify-send "OhPDF" "Export complete."'

make_nautilus "Add Watermark (Confidential)" \
'for f in $NAUTILUS_SCRIPT_SELECTED_FILE_PATHS; do
  [[ "$f" == *.pdf ]] || continue
  out="${f%.pdf}_watermarked.pdf"
  '"$OHPDF"' watermark "$f" "$out" -t Confidential &
done; wait; notify-send "OhPDF" "Watermark applied."'

# ── Nemo actions ──────────────────────────────────────────────────────────────
NEMO_DIR="$HOME/.local/share/nemo/actions"
mkdir -p "$NEMO_DIR"

cat > "$NEMO_DIR/ohpdf-compress.nemo_action" << 'NEMO'
[Nemo Action]
Name=OhPDF — Compress
Comment=Reduce PDF file size
Exec=/usr/local/bin/ohpdf compress %F
Icon-Name=document-save
Selection=Any
Extensions=pdf;
NEMO

cat > "$NEMO_DIR/ohpdf-to-images.nemo_action" << 'NEMO'
[Nemo Action]
Name=OhPDF — Export to Images
Comment=Render each PDF page as PNG
Exec=/usr/local/bin/ohpdf to-images %F
Icon-Name=image-x-generic
Selection=Any
Extensions=pdf;
NEMO

echo "  ✓ Nemo actions installed"

echo ""
echo "Done!"
echo "Nautilus: right-click a PDF → Scripts → OhPDF"
echo "Nemo:     right-click a PDF → OhPDF"
