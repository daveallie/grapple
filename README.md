[![Build Status](https://travis-ci.org/daveallie/grapple.svg?branch=master)](https://travis-ci.org/daveallie/grapple)

# Grapple

Interruptible, download accelerator, with Basic and Digest Authentication support, written in Rust.

![grapple usage](docs/grapple.gif)

## Installation

### Installation through cargo

1. Install [Rustup](https://rustup.rs/)
2. Run
```
cargo install --git https://github.com/daveallie/grapple
```

### Installing binary manually

1. Download the zipped binary for your platform from the [latest release](https://github.com/daveallie/grapple/releases/latest) page.
2. Copy or symlink the binary to `/usr/local/bin` or place it on your `PATH`.

## Usage

```
$ grapple --help
Grapple 0.3.0
Dave Allie
Fast, interruptible file downloader in Rust

USAGE:
    grapple [OPTIONS] <URI>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --parts <PARTS>
            Set part count, defaults to the thread count. Cannot be less than the thread count.

    -P, --password <PASSWORD>                    Password to use in authentication. Overrides password if set in URI.
        --thread-bandwidth <THREAD_BANDWIDTH>    Per thread bankdwidth in kB/s
    -t, --threads <THREADS>                      Set thread count, defaults to 10.
    -u, --username <USERNAME>                    Username to use in authentication. Overrides username in URI.

ARGS:
    <URI>    URI of file to download
```

## Contributing

1. Fork it!
- Create your feature branch: `git checkout -b my-new-feature`
- Commit your changes: `git commit -am 'Add some feature'`
- Push to the branch: `git push origin my-new-feature`
- Submit a pull request :D

### Development

1. Install [Vagrant](https://www.vagrantup.com/downloads.html)
- Navigate to the development directory
- Run `vagrant up`
- Run `vagrant ssh`
- Project will be in the `~/grapple` folder
- Run `cargo build` to build the source

## License

The project is available as open source under the terms of the [MIT License](http://opensource.org/licenses/MIT).
