# gestetner

A netcat and HTTP command line pastebin for sharing straight from your terminal.

## Why another one?
I was able to find pastebin servers which supported publishing pastes over plain sockets (with `nc`) or
over HTTP(S) with `curl`, but not one that supported both!

There are some scenarios where all you have available is a shell, and being able to [paste to a plain socket](https://github.com/solusipse/fiche#pure-bash-alternative-to-netcat)
is surprisingly useful.

## Client Usage
```
SYNOPSIS
    <command> | nc etc.fstab.me 9999
    <command> | curl --data-binary @- https://etc.fstab.me

EXAMPLES
    ~$ ls -l | curl --data-binary @- https://etc.fstab.me
       https://etc.fstab.me/abcd

    ~$ ls -l | nc etc.fstab.me 9999
        https://etc.fstab.me/efgh
```

## Server Usage
```
USAGE:
  gestetner -l '[::]:9999' -w '[::]:8080' -p /tmp/gst -u http://localhost:8080

FLAGS:
  -h, --help        Prints help information

OPTIONS:
  -u URL            Set the base URL to be returned in paste responses
  -l HOST:PORT      Set the listening socket address for incoming pastes
  -p PATH           Set the filesystem path in which to store pastes
  -w HOST:PORT      Set the listening socket for the HTTP server

  -n LENGTH         Set the length of the random paste slug (default: 4)
  -m MAX_SIZE       Set the maximum size of a paste in bytes (default: 512KiB)
  -r RATE           Maximum number of pastes per minute from a single IP (default: 5)
  --capacity SIZE   Set the maximum size of the paste directory (default: 100MiB)
```

Gestetner has built-in rate limiting, a paste size limiter, and will remove the oldest pastes in order to keep its disk
usage below the value you supply for `capacity`.

## Installation

```
$ git clone git@github.com:bradfier/gestetner.git

$ cargo build --release
$ sudo cp target/release/gestetner /usr/local/bin/

$ gestetner -l 127.0.0.1:9999 -w 127.0.0.1:8080 -p /tmp -u http://localhost:8080
```

Alternatively, install from Crates.io with `cargo install gestetner` or use the provided `Dockerfile` to build an image instead.

## License
`gestetner` is licensed under the GNU Affero General Public License, Version 3.
See [LICENSE](LICENSE) for more information.
