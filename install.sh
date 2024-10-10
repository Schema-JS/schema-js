#!/bin/sh
# Copyright 2024 the SchemaJS authors. All rights reserved. MIT license.
# Adapted from https://deno.land/x/install@v0.3.1/install.sh by the Deno authors.
# TODO(everyone): Keep this script simple and easily auditable.

set -e

if ! command -v unzip >/dev/null && ! command -v 7z >/dev/null; then
	echo "Error: either unzip or 7z is required to install SchemaJS." 1>&2
	exit 1
fi

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-pc-windows-msvc"
else
	case $(uname -sm) in
	"Darwin x86_64") target="x86_64-apple-darwin" ;;
	"Darwin arm64") target="aarch64-apple-darwin" ;;
	"Linux aarch64") target="aarch64-unknown-linux-gnu" ;;
	*) target="x86_64-unknown-linux-gnu" ;;
	esac
fi

print_help_and_exit() {
	echo "Setup script for installing deno

Options:
  -y, --yes
    Skip interactive prompts and accept defaults
  --no-modify-path
    Don't add deno to the PATH environment variable
  -h, --help
    Print help
"
	echo "Note: Deno was not installed"
	exit 0
}

# Simple arg parsing - look for help flag, otherwise
# ignore args starting with '-' and take the first
# positional arg as the deno version to install
for arg in "$@"; do
	case "$arg" in
	"-h")
		print_help_and_exit
		;;
	"--help")
		print_help_and_exit
		;;
	"-"*) ;;
	*)
		if [ -z "$sjs_version" ]; then
			sjs_version="$arg"
		fi
		;;
	esac
done
if [ -z "$sjs_version" ]; then
	sjs_version="0.1.0"
fi


sjs_uri="https://github.com/Schema-JS/schema-js/releases/download/v${sjs_version}/schemajs-${target}.zip"
sjs_install="${DENO_INSTALL:-$HOME/.schemajs}"
bin_dir="$sjs_install/bin"
exe="$bin_dir/schemajs"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

curl --fail --location --progress-bar --output "$exe.zip" "$sjs_uri"
if command -v unzip >/dev/null; then
	unzip -d "$bin_dir" -o "$exe.zip"
else
	7z x -o"$bin_dir" -y "$exe.zip"
fi
chmod +x "$exe"
rm "$exe.zip"

echo "SchemaJS was installed successfully to $exe"
echo
echo "Stuck? Join our Discord https://discord.gg/nRzTHygKn5"