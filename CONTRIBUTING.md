# `rustdoc` contribution guidelines

Thank you for your interest in making `rustdoc` better! We'd love to have your
contribution. We expect all contributors to abide by the [Rust code of
conduct](https://www.rust-lang.org/en-US/conduct.html), which you can find at
that link or in the [`CODE_OF_CONDUCT.md`](https://github.com/steveklabnik/rustdoc/blob/master/CODE_OF_CONDUCT.md) file in this repository.

## License

`rustdoc` is dual licenced under the MIT and Apache 2.0 licenses, and so are all
contributions. Please see the [`LICENSE-MIT`](https://github.com/steveklabnik/rustdoc/blob/master/LICENSE-MIT) and [`LICENSE-APACHE`](https://github.com/steveklabnik/rustdoc/blob/master/LICENSE-APACHE) files in this directory for more
details.

## Pull Requests

To make changes to `rustdoc`, please send in pull requests on GitHub to the
`master` branch. We'll review them and either merge or request changes. Travis
CI tests everything as well, so you may get feedback from it too.

If you make additions or other changes to a pull request, feel free to either amend
previous commits or only add new ones, however you prefer. We may ask you to squash
your commits before merging, depending.

## Issue Tracker

You can find the issue tracker [on
GitHub](https://github.com/steveklabnik/rustdoc/issues). If you've found a problem with
`rustdoc`, please open an issue there.

We have several tags for issues:

 5 labels

* **help wanted** are issues explicitly marked for new contributors to work on.
  We'll help answer any questions you might have. If you've got your eyes on a
  ticket that doesn't have this tag, feel free to tackle it if you'd prefer!
  This is just a suggestion.
* **quest** issues are "big picture" things that are fairly open-ended, and are
  just waiting for some enthusiastic contributor to come along and knock them
  out! It's a lot of work, but you also have the chance to make a huge impact.
* **question** issues are for getting help with `rustdoc` in some way.
* **bug** issues track something `rustdoc` isn't doing correctly.
* **enhancement** issues keep track of things `rustdoc` should be doing that it
  doesn't do quite just yet.
* **refactoring** issues track work we can do to clean up the code a bit.
* **blocked on upstream** means that this bug is waiting on something in another
  repository; that might be the compiler, `rls-analysis`, or something else.

## Frontend Development Workflow

To work on the front end, you'll need:

* `nodejs` (We use 7.2.1 but any recent version should be fine)
* `npm` (This should come with node, but you may want to upgrade to npm5)

From there, do this:

```bash
> cd frontend
> npm install
```

And you'll get everything else you need, primarily, the `ember` command-line
tool.

We store a sample `data.json` in the `public` directory, so you don't have to worry
about the backend at all to work on the front end; think of it as test data. To
get started:

```bash
> npm start
```

This will open up a copy of the site on [http://localhost:4200/](http://localhost:4200/).
If you're using the sample data, the sample crate is called "example" and so you'll need
to look at [http://localhost:4200/example](http://localhost:4200/example).

This should reload as you change things automatically.

Before sending in a pull request, you'll need to update the vendored assets. First do
this, to remove them:

```bash
> rm -r dist
```

And then do this, to regenerate them:

```bash
> npm run prod
```

You'll see some files in `dist\assets` be removed, and new ones added.

## Backend Development Workflow

To work on the backend, you'll need:

* Rust, specifically, nightly Rust. We hope to move to stable, but cannot just
  yet, as some dependencies require [nightly](https://github.com/rust-lang-nursery/rustup.rs/blob/master/README.md#working-with-nightly-rust).
* Cargo (comes with Rust 99% of the time)

To build the project:

```bash
> cargo build
```

Your first build will take longer, because it also has to build the
dependencies.

To run tests:

```bash
> cargo test
```

To test out rustdoc's functionality, you'll want to run it on another crate.
The `--manifest-path` flag is a good way to point at the project you want to
try to build docs for. For example, if you were going to build the docs for
the `example` crate, located in this repo, you'd type this:

```bash
> cargo run -- --manifest-path=example build
```

"build" is the subcommand to build the docs, and `--manifest-path` should point
to a directory where a `Cargo.toml` lives.
