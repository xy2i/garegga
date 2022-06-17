# builds and runs the barebones kernel in qemu

#set -x

# 1. build the kernel
# Cargo test binaries names are random: the filename is {binary}-{build_hash}.
# This is an open Rust issue: https://github.com/rust-lang/cargo/issues/1924
# In the meantime, use the trick from https://github.com/rust-lang/cargo/issues/1924#issuecomment-289770287
TEST_BINARY_ABSOLUTE_PATH=$(cargo test --no-run --message-format=json | jq -r "select(.profile.test == true) | .filenames[]" )

# 2. fetch and build limine
git clone https://github.com/limine-bootloader/limine.git --branch=v3.0-branch-binary --depth=1
make -C limine

# 3. build the iso file
rm -rf iso_root
mkdir -p iso_root
cp $TEST_BINARY_ABSOLUTE_PATH barebones # get a deterministic name for the binary
cp barebones limine.cfg limine/limine.sys limine/limine-cd.bin limine/limine-cd-efi.bin iso_root/
xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-cd-efi.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o barebones.iso
limine/limine-deploy barebones.iso
rm barebones
rm -rf iso_root

# 4. run the kernel
qemu-system-x86_64 -cdrom barebones.iso --no-reboot -d int -D qemulog.log -serial stdio \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -display none

# Use our return values to see if the test suite  suceeded or not.
# See test::TestResult (in kernel src/test.rs) for why we use these values.
if [[ $? -eq 33 ]]; then
    exit 0
else 
    exit 1
fi