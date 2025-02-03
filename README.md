# bath

## Motivation

`$PATH` is outdated.  
So are a lot of other environment variables and mechanisms that rely on juggling `:` characters to establish implicit
lookup precedence. It's 2025—why are we still managing our toolchains like it's the 90s?

Imagine a world where you can manage different versions of your applications, compiler flags, and linker options *
*properly**—all stored neatly in an **SQLite database**. No more weird shell scripts, no more lost configurations, and
no more wondering why `gcc` just picked up the wrong library version again.

## What does bath do?

Bath is a **TUI-based CLI tool** that helps you **manage, edit, preview, and export** your environment variable
profiles. It brings order to your `$PATH`, compiler flags, and other C/C++-related variables without making you do
mental gymnastics.

### Features:

✅ **Manage $PATH like a sane person**  
✅ **Store multiple environment profiles in SQLite**  
✅ **Interactive TUI with fuzzy search & live previews**  
✅ **Export configurations in a format you can `eval`**  
✅ **Prepend, Append, or Replace mode for any env var**  
✅ **No weird shell scripts, just pure config bliss**

## Installation

```bash
cargo install bath
```

## Running bath

Inside bath, you get:

- **Two Tabs:** One for environment variables, one for profiles (←/→ to switch).
- **Full Control:**
    - `a` → Edit/add a variable (with a **fuzzy search** for types and live export preview).
    - `e` → Modify an existing variable.
    - `d` → Delete a variable.
- **A live preview of the full export command** (ready to be `eval`'d in your shell).

## Exporting profiles

Bath doesn't generate weird scripts with shebangs. It just prints **what you need to eval**, like this:

```bash
➜ bath export my_profile
export PATH="/opt/coolstuff/bin:$PATH"
export CFLAGS="-O2 -Wall"
export LDFLAGS="-L/opt/coolstuff/lib"
```

Or, if you don't specify a profile, you'll get a TUI where you can select one interactively.

The recommended way to use `bath export` is by eval-ing it's output!

```bash
eval $(bath export my_profile)
```

## License

This project is licensed under the GPL-3 license.

