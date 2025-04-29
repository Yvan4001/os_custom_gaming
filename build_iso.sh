#!/bin/bash
set -e
# Variable configuration
PROJECT_NAME="os_gaming" # This should match the package name in the workspace
TARGET_SPEC="x86_64-os_gaming" # Nom de la cible (sans .json)
TARGET_JSON="$TARGET_SPEC.json" # Nom du fichier de configuration de la cible

echo "🔧 Configuring kernel environment..."
# Create temporary configuration file for cargo (if needed, currently empty step)

echo "🚀 Compiling the kernel (verbose)..."
# Execute the library compilation with bootimage, specifying the package and +nightly
# Le +nightly est important ici
cargo +nightly bootimage -vv

# --- ISO Creation Steps ---
echo "📁 Preparing directories..."
# Les chemins sont relatifs à la racine maintenant
BUILD_DIR="$PROJECT_NAME/build" # Chemin relatif au package
ISO_DIR="$BUILD_DIR/iso"
BOOT_DIR="$ISO_DIR/boot"
GRUB_DIR="$BOOT_DIR/grub"
rm -rf $BUILD_DIR # Supprimer le répertoire build relatif au package
mkdir -p $ISO_DIR $BOOT_DIR $GRUB_DIR

echo "📦 Installing files..."
# Copier le binaire du noyau produit par bootimage
# Le chemin de sortie utilise la cible spécifiée dans .cargo/config.toml
cp target/$TARGET_SPEC/debug/bootimage-$PROJECT_NAME.bin $BOOT_DIR/kernel.bin

echo "📄 Preparing system modules (if needed)..."
# (Commentaires inchangés)

echo "💿 Creating the ISO..."
# Le fichier grub.cfg est créé dans os_gaming/build/iso/boot/grub
cat > $GRUB_DIR/grub.cfg << EOF
set timeout=3
set default=0
menuentry "OS Gaming" {
    multiboot /boot/kernel.bin
    # module /boot/system.bin System # Décommenter si nécessaire
    # module /boot/config.bin Config # Décommenter si nécessaire
    boot
}
EOF

# grub-mkrescue s'exécute depuis la racine, pointant vers os_gaming/build/iso
# L'ISO est créée à la racine du workspace
if command -v grub-mkrescue &> /dev/null; then
    grub-mkrescue -o $PROJECT_NAME.iso $ISO_DIR
else
    echo "❌ ERROR: grub-mkrescue not found."
    exit 1
fi
# L'ISO est maintenant à la racine
echo "✅ ISO created successfully: $PROJECT_NAME.iso (at workspace root)"
# --- End of ISO Creation Steps ---

echo "✨ Build process finished."