# Warning: Experimental source dump

This project was never fully implemented, this is a work-in-progress dump of a
project I've been working on in fall 2020. The ECH feature has never been
implemented though.

Working so far:

- Hooking connections from signal-desktop with proxychains-ng and accepting
  them with a socks5 server so we're in control how the connection is made
- Resolving the ip of the proxy front with dns-over-https
- Creating a standard tls connection (with an **unencrypted** SNI extension)
- Forwarding the connection request through a websocket protocol
- Accepting connection requests with a websocket server that proxies them into
  the internet

Link preview connections are established directly instead of forwarding them
because the remote proxy is likely using an allow-list to only accept
signal.org traffic. The websocket server is meant to be proxied through the
servers of a content delivery network. The websocket url may co-exist on an
existing site and making the path configurable would add probing resistance.

Due to recent events in Iran I've decided to dump the source code on github.

---

# signal-doh-ech

- hook into signal-desktop to redirect everything into a socks5 proxy provided
  by signal-doh-ech.
- the socks proxy uses doh and tls1.3 with ech to create an encrypted tunnel
  that doesn't leak any metadata besides the layer 3 address and traffic
  patterns.
- use a simple webhook protocol to connect to a remote proxy server to be
  compatible with CDN limitations.
- connect to the signal infrastructure through this covert tunnel.

## Setup

    git clone https://github.com/kpcyrd/signal-doh-ech.git
    cd signal-doh-ech
    cargo +nightly install -f --path .
    signal-doh-ech --help

## Usage (local)

    signal-doh-ech tunnel -v --bind 127.0.0.1:1090 --proxy todo.example.com \
        -F textsecure-service.whispersystems.org:443 -F storage.signal.org:443 -F cdn.signal.org:443 \
        -F cdn2.signal.org:443 -F api.directory.signal.org:443 -F contentproxy.signal.org:443 \
        -F uptime.signal.org:443 -F api.backup.signal.org:443 -F sfu.voip.signal.org:443 \
        -F updates.signal.org:443 -F updates2.signal.org:443

## Running signal

At the time of writing, this requires
[proxychains-ng-git](https://aur.archlinux.org/packages/proxychains-ng-git/)
until
[7fe8139](https://github.com/rofl0r/proxychains-ng/commit/7fe813949644b115b0127279517dc7c0ee2d63b9)
is released:

    proxychains4-daemon &
    proxychains -f proxychains.conf signal-desktop

## Usage (server)

    signal-doh-ech backend -v \
        -A textsecure-service.whispersystems.org:443 -A storage.signal.org:443 -A cdn.signal.org:443 \
        -A cdn2.signal.org:443 -A api.directory.signal.org:443 -A contentproxy.signal.org:443 \
        -A uptime.signal.org:443 -A api.backup.signal.org:443 -A sfu.voip.signal.org:443 \
        -A updates.signal.org:443 -A updates2.signal.org:443

This binds a websocket server to `127.0.0.1:3030`. You also need to setup nginx
and configure https. See
[acme-redirect](https://github.com/kpcyrd/acme-redirect) for certificates.

```
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;

    server_name example.com;

    ssl_certificate /var/lib/acme-redirect/live/EXAMPLE.COM/live/fullchain;
    ssl_certificate_key /var/lib/acme-redirect/live/EXAMPLE.COM/live/privkey;
    ssl_session_timeout 1d;
    ssl_session_cache shared:MozSSL:10m;  # about 40000 sessions
    ssl_session_tickets off;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    ssl_stapling on;
    ssl_stapling_verify on;
    ssl_trusted_certificate /var/lib/acme-redirect/live/EXAMPLE.COM/chain;
    resolver 127.0.0.1;

    location /connect {
        proxy_pass http://127.0.0.1:3030;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header Host $host;
    }
}
```

If you don't configure ECH we would fall back to regular domain fronting. The
easiest way to get a working setup with ECH is configuring cloudflare for this
proxy.

When self-hosting without a CDN or similar as a layer3 front you're likely
subverting the benefits of ECH and you need to keep the server name and ip
secret instead of annoucing them publicly to prevent deny-listing. For an
effective front you should have a) popular sites to provide deniability and b)
have sites that are important enough to make a blanket ban of the whole ip
unappealing to the censor.

## Limitations

The suggested profile intends to bypass deny-lists but doesn't attempt to 100%
hide signal usage. If a new signal endpoint is introduced in signal-desktop you
need to update your configuration. You can attempt to forward all traffic with
`-F '*'` but this also tunnels link previews, which the remote proxy might
reject.

## TODO

- auto ping

## Development

    cargo +nightly run -- tunnel -vv --bind 127.0.0.1:1090 --proxy 127.0.0.1 --proxy-port 3030 --skip-tls -F example.com:443 -F google.com:443'
    cargo +nightly run -- backend -vv -A example.com:443
    curl -vx socks5h://127.0.0.1:1090 https://github.com
