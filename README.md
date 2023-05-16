# format-aliases

This is a reimplementation of the [OhMyZsh alias cheatsheet plugin](https://github.com/ohmyzsh/ohmyzsh/tree/master/plugins/aliases).

## Setup

Run the `format-aliases` init sequence in your shell's rc file. A new function will be included in your shell that overrides the `alias` builtin.

```sh
# Posix shell example
alias() {
    if [ $# -eq 0 ]; then
        # Pipe the output of the builtin alias command to be formatted
        builtin alias | format-aliases
    else
        # Pass the arguments to the builtin alias command
        builtin alias ${@}
    fi
}
```

### Init sequences by shell

#### Zsh

```zsh
eval "$(format-aliases init zsh)"
```

#### Bash

```bash
eval "$(format-aliases init bash)"
```

#### Bourne Shell (sh)

```sh
eval "$(format-aliases init sh)"
```

## Usage

Run the new `alias` command without any args to display all active aliases. Running with input args will pass args to the shell's `alias` builtin.

### Color control

Printing in color is enabled by default. Set env var `NO_COLOR` to disable.

```bash
NO_COLOR alias
```
