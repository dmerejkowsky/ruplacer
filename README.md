[![crates.io image](https://img.shields.io/crates/v/ruplacer.svg)](https://crates.io/crates/ruplacer)

# Ruplacer

Find and replace text in source files:

```
$ ruplacer old new src/
Patching src/a_dir/sub/foo.txt
-- old is everywhere, old is old
++ new is everywhere, new is new

Patching src/top.txt
-- old is nice
++ new is nice

Would perform 2 replacements on 2 matching files.
Re-run ruplacer with --go to write these changes to disk
```

## Note

This project was originally hosted on the
[TankerHQ](https://github.com/TankerHQ) GitHub organization, which was
my employer from 2016 to 2021. They kindly agreed to give me back ownership
of this project. Thanks!


## Installing with cargo

Install `rust` and `cargo`, for example with [rustup](https://rustup.rs/).

Then run:

```
cargo install ruplacer
```

## Alternative installation methods

* Pre-compiled binaries for Linux, macOS, and Windows are available as [assets of the latest release](
https://github.com/your-tools/ruplacer/releases/tag/v0.10.0).

* `ruplacer` can also be installed from `homebrew`:

```
brew install TankerHQ/homebrew-repo/ruplacer
```

* `ruplacer` is also on [the Arch Linux User Repository](https://aur.archlinux.org/packages/ruplacer/)

## Basic usage

```
ruplacer pattern replacement [path]
```

If the path is not given, it defaults to the current working directory.

Ruplacer will then walk through every file in `<path>` while honoring `.gitignore` files found on the way.

Binary files and text files containing non-UTF8 characters will be skipped. Then for
every remaining file, it will read the contents, replace all lines matching the
pattern by the replacement, and print the difference.

If you are OK with the replacements, re-run `ruplacer` with the `--go` option to actually write the changes to disk.

## Regex

By default, `pattern` will be compiled into a [Rust regex](https://docs.rs/regex/1.0.5/regex/).

Note that it's slightly different from Perl-style regular expressions. Also, you must use `$1`, `$2` to reference
groups captured from `pattern` inside `replacement`.

For instance, this replaces 'last, first' by 'first last':

```
ruplacer '(\w+), (\w+)' '$2 $1'
```

(note the use of single quotes to avoid any processing by the shell)

`${1}` and `${2}` are also supported as an alternative to `$1` and `$2`, see
[reference
here](https://docs.rs/regex/1.5.5/regex/struct.Regex.html#replacement-string-syntax).
Useful if the replacement string is immediately adjacent to literal text.

If you don't want the pattern to be used as a regex, use the `--no-regex` command line flag.

This makes it possible to look for special characters without escaping them:

```
# This is a regex that matches the letter a
# or the letter o
$ ruplacer '(a|o)' u
- tata toto
+ tutu tutu
- (a|o)
+ (u|u)

# This is the literal string: '(a|o)'
$ ruplacer --no-regex '(a|o)' u
# or
$ ruplacer '\(a\|o|)' u
- (a|o)
+ u

```


## Preserving case while replacing

Ruplacer has a `--preserve-case` option which works across a variety of case styles (lower case, snake case, and so on):

```
$ ruplacer --preserve-case foo_bar spam_eggs
Patching src/foo.txt
-- foo_bar, FooBar, and FOO_BAR!
++ spam_eggs, SpamEggs, and SPAM_EGGS!
```

## Filter files by type or glob patterns

Inspired by [ripgrep](https://github.com/BurntSushi/ripgrep), you can also select or ignore certain "file types" or glob patterns:

```
# Select only C++ files
$ ruplacer old new --type cpp
# Select only *.foo files
$ ruplacer old new --type *.foo
# Select only files that match foo*bar.c
$ ruplacer old new --type foo*bar.c

# Ignore all js files
$ ruplacer old new --type-not js
# Ignore all *.bar files
$ ruplacer old new --type-not *.bar
# Ignore all files that match foo*bar.c
$ ruplacer old new --type-not foo*bar.c
```

Each "file type" is just a list of glob pattern. For instance: the `cpp` file type matches `*.C`, `*.H`, `*.cc`, `*.cpp` and so on ...

You can see the whole list by using `ruplacer --type-list`.
