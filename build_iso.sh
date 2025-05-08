#!/bin/bash
set -e
# Variable configuration
PROJECT_NAME="os_gaming" # This should match the package name in the workspace
TARGET_SPEC="x86_64-os_gaming" # Nom de la cible (sans .json)
TARGET_JSON="$TARGET_SPEC.json" # Nom du fichier de configuration de la cible

echo "🔧 Configuring kernel environment..."
# Assurez-vous que les composants nightly sont présents
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu

echo "🚀 Building kernel with build-std..."
# Étape 1: Compiler le noyau avec les flags explicites, comme dans le .bat
# Note: Utilisation de +nightly et $TARGET_JSON
cargo +nightly build \
    -Z build-std=core,alloc,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --target $TARGET_JSON \
    -Z unstable-options \
    -vv # Ajout de verbosité pour le débogage

# Vérifier si la compilation a réussi (bien que set -e devrait le faire)
if [ $? -ne 0 ]; then
    echo "❌ Kernel build failed"
    exit 1
fi

echo "📦 Creating bootimage..."
# Étape 2: Créer l'image de démarrage, en spécifiant la cible
# bootimage utilisera les artefacts de la compilation précédente
cargo +nightly bootimage --target $TARGET_JSON -vv # Ajout de verbosité

if [ $? -ne 0 ]; then
    echo "❌ Bootimage creation failed"
    exit 1
fi

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
# Le chemin de sortie utilise la cible spécifiée
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

echo "💻 To run the ISO in QEMU, use:"
echo "qemu-system-x86_64 -drive format=raw,file=target/x86_64-os_gaming/debug/bootimage-os_gaming.bin"
