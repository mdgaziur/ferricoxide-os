#! /bin/env python3
import os
import shlex
import shutil
import subprocess
import sys

SUPPORTED_ARCHITECTURES = ["x86_64"]

ferricoxide_architecture = None
ferricoxide_boot_assemblies = []
ferricoxide_boot_compiled_assemblies = []

def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)

def get_arch() -> bool:
    arch = os.getenv("FERRICOXIDE_ARCH")
    if arch is None or not arch in SUPPORTED_ARCHITECTURES:
        eprint("FERRICOXIDE_ARCH must be set to a supported architecture: `x86_64`")
        return False

    global ferricoxide_architecture
    ferricoxide_architecture = arch
    return True

def get_boot_assemblies():
    assemblies_dir = f"./kernel/arch/{ferricoxide_architecture}/"

    for file in os.listdir(assemblies_dir):
        filename = os.fsdecode(file)
        if filename.endswith('.s'):
            print(f"Found {assemblies_dir + filename}")
            ferricoxide_boot_assemblies.append(assemblies_dir + filename)

def compile_assembly(path):
    output = f"build/{ferricoxide_architecture}/objects/" + path.split('/')[-1].split('.')[0] + '.o'
    command = f"nasm -felf64 {path} -o {output}"
    print(f"Compiling {path}...")
    subprocess.run(shlex.split(command))
    ferricoxide_boot_compiled_assemblies.append(output)

def compile_assemblies():
    for asm in ferricoxide_boot_assemblies:
        compile_assembly(asm)

def compile_kernel() -> bool:
    command = "cargo build"
    if "--release" in sys.argv:
        command += " --release"

    os.chdir("kernel")
    print("Compiling kernel...")
    process = None
    try:
        process = subprocess.run(shlex.split(command))
    except:
        print("Failed to compile kernel")
        return False

    if process.returncode != 0:
        print("Failed to compile kernel")
        return False

    os.chdir("..")
    return True

def get_build_mode() -> str:
    if "--release" in sys.argv:
        return "release"
    else:
        return "debug"

def link_everything():
    kernel_build_path = f"build/{ferricoxide_architecture}/{ferricoxide_architecture}-ferricoxide_os.bin"
    linker_script = f"./kernel/arch/{ferricoxide_architecture}/linker.ld"
    rust_kernel_path = f"./kernel/target/{ferricoxide_architecture}-ferricoxide_os/{get_build_mode()}/libkernel.a"
    assemblies = ' '.join(ferricoxide_boot_compiled_assemblies)

    print("Linking everything...")
    command = f"ld -n --gc-sections -T {linker_script} -o {kernel_build_path} {assemblies} {rust_kernel_path}"
    subprocess.run(shlex.split(command))

def make_iso():
    print("Making iso...")
    os.makedirs(f"build/{ferricoxide_architecture}/iso_tree/boot/grub")
    shutil.copy(f"build/{ferricoxide_architecture}/{ferricoxide_architecture}-ferricoxide_os.bin", f"build/{ferricoxide_architecture}/iso_tree/boot/")
    shutil.copy(f"kernel/arch/{ferricoxide_architecture}/grub/grub.cfg", f"build/{ferricoxide_architecture}/iso_tree/boot/grub")
    command = f"grub-mkrescue -o build/{ferricoxide_architecture}/{ferricoxide_architecture}-ferricoxide_os.iso build/{ferricoxide_architecture}/iso_tree"
    subprocess.run(shlex.split(command), stderr=subprocess.PIPE)

def boot():
    print("Booting...")
    command = f"qemu-system-{ferricoxide_architecture} -cdrom build/{ferricoxide_architecture}/{ferricoxide_architecture}-ferricoxide_os.iso -d cpu_reset -serial stdio -no-reboot -no-shutdown -s"
    subprocess.run(shlex.split(command))

def format_kernel_code():
    command = "cargo fmt"
    os.chdir("kernel")
    subprocess.run(shlex.split(command))
    os.chdir("..")

def fix_kernel_code():
    command = "cargo clippy --fix --allow-dirty --allow-staged --lib -p kernel"
    os.chdir("kernel")
    subprocess.run(shlex.split(command))
    os.chdir("..")

    command = "cargo fix --allow-dirty --allow-staged --lib -p kernel"
    os.chdir("kernel")
    subprocess.run(shlex.split(command))
    os.chdir("..")

def main():
    if "format" in sys.argv:
        format_kernel_code()
        return
    if "fix" in sys.argv:
        fix_kernel_code()
        return

    if not get_arch():
        return

    if os.path.isdir("build"):
        shutil.rmtree(f"build/{ferricoxide_architecture}")

    os.makedirs(f"build/{ferricoxide_architecture}/objects")

    get_boot_assemblies()
    compile_assemblies()
    if not compile_kernel():
        return
    link_everything()
    make_iso()

    if "run" in sys.argv:
        boot()

if __name__ == "__main__":
    main()
