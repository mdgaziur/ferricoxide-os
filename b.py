#! /bin/env python3

#  FerricOxide OS is an operating system that aims to be posix compliant and memory safe
#  Copyright (C) 2024  MD Gaziur Rahman Noor
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.

import argparse
import os
import shlex
import shutil
import subprocess
import sys
from enum import Enum

SUPPORTED_ARCHITECTURES = ["x86_64"]
ARCHITECTURE_INFO = {
    "x86_64": {
        "boot": "x86",
        "prekernel_target": "i686",
        "prekernel_rust_builddir": "i686-ferricoxide_os",
        "kernel_rust_builddir": "x86_64-ferricoxide_os",
        "prekernel_assemblies": ["prekernel/arch/x86/boot"],
        "kernel_assemblies": ["kernel/arch/x86_64"],
        "prekernel_link_arch": "elf_i386",
        "kernel_link_arch": "elf_x86_64",
        "prekernel_linker_script": "prekernel/arch/x86/linker.ld",
        "kernel_linker_script": "kernel/arch/x86_64/linker.ld",
        "prekernel_entry": "start",
        "kernel_entry": "kernel_start",
        "prekernel_asm_arch": "elf32",
        "kernel_asm_arch": "elf64",
    }
}


class BuildMode(Enum):
    RELEASE = 0
    DEBUG = 0


build_config = {
    "arch": SUPPORTED_ARCHITECTURES[0],
    "memory": 128,
    "fix": False,
    "reformat": True,
    "build_mode": BuildMode.DEBUG,
    "build": False,
    "boot": False,
    "extra_qemu_args": "",
}


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, *kwargs)


def execute_command(operation: str, cmd: str, exit_on_failure: bool = False):
    return_code = 0

    try:
        subprocess.run(shlex.split(cmd), check=True)
    except subprocess.CalledProcessError as e:
        return_code = e.returncode
    except FileNotFoundError:
        return_code = -1
    except KeyboardInterrupt:
        eprint(f"\033[91mReceived keyboard interrupt while performing task `{operation}`. Bye!\033[0m")
        exit(1)

    if return_code == -1:
        eprint(f"\033[91mFailed to {operation}. This indicates that the required program is missing on your "
               f"machine.\033[0m")
    elif return_code != 0:
        eprint(f"\033[91mFailed to {operation}. Return code: {return_code}\033[0m")

        if exit_on_failure:
            exit(1)


def reformat_code(directory: str):
    print(f"Reformatting code at `{directory}`...")
    cur_dir = os.path.abspath(os.path.curdir)
    os.chdir(directory)
    execute_command(f"reformat code at `{directory}`", "cargo fmt", False)
    os.chdir(cur_dir)


def fix_code(directory: str):
    print(f"Fixing code at `{directory}`...")
    cur_dir = os.path.abspath(os.path.curdir)
    os.chdir(directory)
    execute_command(f"fix code at `{directory}`", "cargo clippy --fix  --allow-dirty --allow-staged", False)
    os.chdir(cur_dir)


def build_assemblies(crate: str, directory: str, build_dir):
    print(f"Building assemblies at `{directory}`...")
    assemblies = []
    for file in os.listdir(directory):
        if file.split('.')[-1] == 'asm':
            print(f"- Found assembly at `{directory}/{file}`")
            assemblies.append(file)

    for assembly in assemblies:
        print(f"- Assembling assembly: `{directory}/{assembly}`")
        execute_command(f"assemble assembly at `{directory}/{assembly}`",
                        f"nasm -f{ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_asm_arch"]} {directory}/{assembly} -o {build_dir}/{''.join(assembly.split('.')[:-1])}.o", True)


def link_crate(crate: str, output: str, build_dir: str):
    files = map(lambda f: build_dir + "/" + f, os.listdir(build_dir))

    print(f"Linking `{crate}`...")
    execute_command(f"link {crate}",
                    f"ld -m {ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_link_arch"]} -n "
                    f"--gc-sections -T {ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_linker_script"]}"
                    f" -o {output} {' '.join(files)} --entry {ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_entry"]}",
                    True)


def build_crate(crate: str, release: bool, build_dir: str):
    print(f"Building crate at `{crate}`...")
    cur_dir = os.path.abspath(os.path.curdir)
    os.chdir(crate)

    command = "cargo build"
    if release:
        command += " --release"

    execute_command(f"build crate at `{crate}`", command, True)

    os.chdir(cur_dir)

    shutil.copy(
        f"{crate}/target/"
        f"{ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_rust_builddir"]}/"
        f"{"release" if build_config["release"] else "debug"}/lib{crate}.a",
        build_dir)

    for assembly_dir in ARCHITECTURE_INFO[build_config["architecture"]][f"{crate}_assemblies"]:
        build_assemblies(crate, assembly_dir,
                         build_dir)

    link_crate(crate, f"{build_dir}/{crate}.bin", build_dir)


def turn_kernel_into_binary_object(build_dir: str):
    print("Turning `kernel.bin` into a binary object...")
    cur_dir = os.path.abspath(os.path.curdir)
    os.chdir(f"{build_dir}/kernel")

    execute_command(
        "turn `kernel.bin` into a binary object",
        f"objcopy kernel.bin kernel.o -I binary -B i386 -O elf32-i386",
        True
    )

    shutil.copy("kernel.o", "../prekernel")

    os.chdir(cur_dir)


def make_iso():
    print("Making iso...")
    os.makedirs(f"build/{build_config["architecture"]}/iso_tree/boot/grub")
    shutil.copy(f"build/{build_config["architecture"]}/prekernel/prekernel.bin",
                f"build/{build_config["architecture"]}/iso_tree/boot/"
                f"{build_config["architecture"]}-ferricoxide_os.bin")
    shutil.copy(f"prekernel/arch/{ARCHITECTURE_INFO[build_config["architecture"]]["boot"]}/boot/grub.cfg",
                f"build/{build_config["architecture"]}/iso_tree/boot/grub")
    command = (f"grub-mkrescue -o build/{build_config["architecture"]}/"
               f"{build_config["architecture"]}-ferricoxide_os.iso build/{build_config["architecture"]}/iso_tree")

    execute_command(
        "create iso",
        command,
        True
    )


def boot():
    iso_path = f"build/{build_config["architecture"]}/{build_config["architecture"]}-ferricoxide_os.iso"
    if not os.path.exists(iso_path):
        eprint("\033[91mRun with --build first.\033[0m")
        return

    print(f"Booting from `{iso_path}`...")
    command = (f"qemu-system-{build_config["architecture"]} -m {build_config["memory"]}M "
               f"-cdrom {iso_path} "
               f"-d cpu_reset  -serial stdio -no-reboot -no-shutdown -s " + build_config["extra_qemu_args"])

    execute_command(
        "boot",
        command,
        True
    )


def main():
    parser = argparse.ArgumentParser(
        prog='b.py',
        description='Build tool for the Ferricoxide Operating System'
    )

    parser.add_argument("--architecture", help="target architecture", default="x86_64", choices=SUPPORTED_ARCHITECTURES)
    parser.add_argument("--release", help="build with optimizations enabled", action='store_true')
    parser.add_argument("--build", help="compile Ferricoxide OS", action='store_true')
    parser.add_argument("--boot", help="boot Ferricoxide OS", action='store_true')
    parser.add_argument("--reformat", help="reformat code", action='store_true')
    parser.add_argument("--fix", help="fix lint errors", action='store_true')
    parser.add_argument("-m", "--memory", help="total memory given to the OS", default=128)
    parser.add_argument("-q", "--extra-qemu-args", help="extra arguments for QEMU(should be escaped)")

    args = parser.parse_args()

    if args.release:
        build_config["build_mode"] = BuildMode.RELEASE

    build_config["architecture"] = args.architecture
    build_config["build"] = args.build
    build_config["boot"] = args.boot
    build_config["reformat"] = args.reformat
    build_config["fix"] = args.fix
    build_config["memory"] = args.memory
    build_config["release"] = args.release
    build_config["extra_qemu_args"] = args.extra_qemu_args if args.extra_qemu_args else ""

    if build_config["reformat"]:
        reformat_code("prekernel")
        reformat_code("kernel")
        return
    elif build_config["fix"]:
        fix_code("prekernel")
        fix_code("kernel")
        return

    if build_config["build"]:
        build_dir = f"build/{build_config["architecture"]}"
        if os.path.exists(build_dir):
            print(f"Build dir `{build_dir}` already exists, removing it")
            shutil.rmtree(build_dir)
        os.makedirs(build_dir)

        os.mkdir(build_dir + "/prekernel")
        os.mkdir(build_dir + "/kernel")

        build_crate("kernel", build_config["release"], build_dir + "/kernel")
        turn_kernel_into_binary_object(build_dir)

        build_crate("prekernel", build_config["release"], build_dir + "/prekernel")

        make_iso()

    if build_config["boot"]:
        boot()

    if build_config["boot"] or build_config["build"]:
        return

    parser.print_usage()
    print("\nNothing to do. Bye!")


if __name__ == "__main__":
    main()
