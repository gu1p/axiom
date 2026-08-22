#!/bin/sh

set -eu

repository="${AXIOM_REPOSITORY:-gu1p/axiom}"
install_dir="${AXIOM_INSTALL_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"

fail() {
    printf 'axiom installer: %s\n' "$1" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

case "$(uname -s)" in
    Darwin) platform="apple-darwin" ;;
    Linux) platform="unknown-linux-musl" ;;
    *) fail "unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) architecture="x86_64" ;;
    arm64 | aarch64) architecture="aarch64" ;;
    *) fail "unsupported architecture: $(uname -m)" ;;
esac

tag="${AXIOM_VERSION:-}"
if [ -z "$tag" ]; then
    latest_url="$(curl --proto '=https' --tlsv1.2 -fsSL -o /dev/null \
        -w '%{url_effective}' "https://github.com/$repository/releases/latest")"
    tag="${latest_url##*/}"
else
    case "$tag" in
        v*) ;;
        *) tag="v$tag" ;;
    esac
fi

case "$tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *) fail "GitHub returned an invalid release tag: $tag" ;;
esac

target="$architecture-$platform"
archive="axiom-$tag-$target.tar.gz"
download_url="https://github.com/$repository/releases/download/$tag"
temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/axiom-install.XXXXXX")"
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

printf 'Downloading Axiom %s for %s...\n' "$tag" "$target"
curl --proto '=https' --tlsv1.2 -fsSL "$download_url/$archive" \
    -o "$temporary_dir/$archive"
curl --proto '=https' --tlsv1.2 -fsSL "$download_url/$archive.sha256" \
    -o "$temporary_dir/$archive.sha256"

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$temporary_dir" && sha256sum -c "$archive.sha256")
elif command -v shasum >/dev/null 2>&1; then
    (cd "$temporary_dir" && shasum -a 256 -c "$archive.sha256")
else
    fail "sha256sum or shasum is required to verify the download"
fi

tar -xzf "$temporary_dir/$archive" -C "$temporary_dir"
binary="$temporary_dir/${archive%.tar.gz}/axiom"
[ -f "$binary" ] || fail "release archive does not contain axiom"

mkdir -p "$install_dir"
install -m 0755 "$binary" "$install_dir/axiom"
installed_version="$("$install_dir/axiom" --version)"
[ "$installed_version" = "axiom ${tag#v}" ] || fail "installed version is $installed_version"

path_changed=false
if [ "${AXIOM_NO_MODIFY_PATH:-0}" != "1" ]; then
    profile="${AXIOM_PROFILE:-}"
    shell_name="${SHELL##*/}"
    if [ -z "$profile" ]; then
        case "$shell_name" in
            zsh) profile="$HOME/.zshrc" ;;
            bash)
                if [ "$(uname -s)" = "Darwin" ]; then
                    profile="$HOME/.bash_profile"
                else
                    profile="$HOME/.bashrc"
                fi
                ;;
            fish) profile="$HOME/.config/fish/config.fish" ;;
            *) profile="$HOME/.profile" ;;
        esac
    fi

    mkdir -p "$(dirname "$profile")"
    touch "$profile"
    if [ "$shell_name" = "fish" ]; then
        path_line="fish_add_path \"$install_dir\""
    else
        path_line="export PATH=\"$install_dir:\$PATH\""
    fi
    if ! grep -Fqx "$path_line" "$profile"; then
        printf '\n%s\n' "$path_line" >> "$profile"
        path_changed=true
    fi
fi

printf 'Installed %s at %s/axiom\n' "$installed_version" "$install_dir"
if [ "$path_changed" = true ]; then
    printf 'Added %s to PATH; restart your shell to use axiom.\n' "$install_dir"
elif ! command -v axiom >/dev/null 2>&1; then
    printf 'Add %s to PATH to use axiom.\n' "$install_dir"
fi
