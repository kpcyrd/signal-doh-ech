# signal-doh-ech


## Usage (server)

    signal-doh-ech backend -v \
        -A textsecure-service.whispersystems.org:443 -A cdn.signal.org:443 -A storage.signal.org:443

## Usage (local)

    signal-doh-ech tunnel -v --bind 127.0.0.1:1090 --proxy 127.0.0.1 --proxy-port 3030 --skip-tls \
        -F textsecure-service.whispersystems.org:443 -F cdn.signal.org:443 -F storage.signal.org:443

## Running signal

At the time of writing, this requires [proxychains-ng-git](https://aur.archlinux.org/packages/proxychains-ng-git/):

    proxychains4-daemon &
    proxychains -f proxychains.conf signal-desktop

## TODO

- auto ping

## Development

    cargo run -- tunnel -vv --bind 127.0.0.1:1090 --proxy 127.0.0.1 --proxy-port 3030 --skip-tls -F example.com:443 -F google.com:443'
    cargo run -- backend -vv -A example.com:443
    curl -vx socks5h://127.0.0.1:1090 https://github.com
