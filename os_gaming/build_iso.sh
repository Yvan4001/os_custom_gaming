#!/bin/bash
set -e

# Configuration des variables
KERNEL_ENTRY="kernel/boot.rs"  # Point d'entrée du kernel
PROJECT_NAME="os_gaming"
TARGET_SPEC="x86_64-unknown-none"

# Suppression de la configuration cargo existante
rm -f .cargo/config.toml

echo "🔧 Configuration environnement of kernel..."
# Création du fichier de configuration temporaire pour cargo
cat > .cargo/config.toml << EOF
[build]
target = "${TARGET_SPEC}"

[unstable]
build-std = ["core", "compiler_builtins", "alloc"]
build-std-features = ["compiler-builtins-mem"]

[target.'cfg(target_os = "none")']
runner = "bootimage runner"

[target.${TARGET_SPEC}]
rustflags = [
    "-C", "link-arg=-nostartfiles",
    "-C", "link-arg=-nodefaultlibs",
    "-C", "link-arg=-nostdlib",
    "-C", "link-arg=-T./linker.ld",
    "-C", "target-feature=+crt-static"
]
EOF


# Vérification de la version de serde
echo "🔍 Vérification des dépendances..."
cargo tree -i serde --target $TARGET_SPEC | grep "1.0.152" || {
    echo "❌ ERROR: Mauvaise version de serde pour le build kernel!";
    exit 1;
}

# Configuration des flags pour la compilation
export RUSTFLAGS="-C link-arg=-nostartfiles \
                  -C link-arg=-nodefaultlibs \
                  -C link-arg=-nostdlib \
                  -C link-arg=-T./linker.ld \
                  -C target-feature=+crt-static"

echo "🚀 Compilation du kernel..."
cargo +nightly build --target x86_64-unknown-none -Zbuild-std=core,alloc,compiler_builtins


# Configuration des répertoires
BUILD_DIR="build"
ISO_DIR="$BUILD_DIR/iso"
BOOT_DIR="$ISO_DIR/boot"
GRUB_DIR="$BOOT_DIR/grub"

echo "📁 Préparation des répertoires..."
rm -rf $BUILD_DIR
mkdir -p $ISO_DIR $BOOT_DIR $GRUB_DIR

# Copie du kernel compilé
echo "📦 Installation des fichiers..."
cp target/$TARGET_SPEC/release/$PROJECT_NAME $BOOT_DIR/kernel.bin

# Création de la configuration GRUB
cat > $GRUB_DIR/grub.cfg << EOF
set timeout=3
set default=0

menuentry "OS Gaming" {
    multiboot /boot/kernel.bin
    module /boot/system.bin System
    module /boot/config.bin Config
    boot
}
EOF

# Copie des fichiers système additionnels
echo "📄 Préparation des modules système..."
# Compilation et copie du system.rs
cargo +nightly rustc --release -- --emit=obj -o $BOOT_DIR/system.bin src/system.rs
# Compilation de la configuration
cargo +nightly rustc --release -- --emit=obj -o $BOOT_DIR/config.bin src/config/mod.rs

echo "💿 Création de l'ISO..."
if command -v grub-mkrescue &> /dev/null; then
    grub-mkrescue -o $PROJECT_NAME.iso $ISO_DIR
else
    echo "❌ ERROR: grub-mkrescue non trouvé. Installez-le avec:"
    echo "sudo apt-get install grub-common xorriso"
    exit 1
fi

echo "✅ ISO créée avec succès: $PROJECT_NAME.iso"

# Nettoyage
rm -f .cargo/config.toml

echo "🔍 Vérification de l'ISO..."
if [ -f "$PROJECT_NAME.iso" ]; then
    echo "✨ Build terminé avec succès!"
    echo "Pour tester l'ISO:"
    echo "qemu-system-x86_64 -cdrom $PROJECT_NAME.iso -m 1G"
else
    echo "❌ Échec de la création de l'ISO"
    exit 1
fi