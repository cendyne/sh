Compiling for x86_64 linux on mac is difficult.

```
rustup target add x86_64-unknown-linux-gnu
brew tap SergioBenitez/osxct
brew install x86_64-unknown-linux-gnu
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo build --release --target=x86_64-unknown-linux-gnu
```

an `ALPHABET` environment variable for what characters would be used in generated urls

```
node
Welcome to Node.js v16.13.1.
Type ".help" for more information.
> str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
```

a `TOKEN` environment variable is the authorization bearer token.
You can use openssl to generate an unguessable one
```
openssl rand -base64 33
K9h69DfTsTjVPGUrgg2o3fv5hjIglNoIfci1+kMFfyAg
```

a `PORT` variable supplies what port to use upon start up, by default 8080.
It should bind to all interfaces by binding to `0.0.0.0`

a `DATA` variable supplies what json file should be used to store data.
This is a very simple server, it will rewrite the json file every time new links are added.

a `INCREMENT_SECRET` variable makes the next short url unguessable
You can use openssl to generate the secret url
```
openssl rand -base64 33
K9h69DfTsTjVPGUrgg2o3fv5hjIglNoIfci1+kMFfyAg
```

This application supports a `.env` file. You can reference [](example.env) and copy it to `.env` with your values.

To create a short link, PUT to `/` on the host with the bearer token.
The body should be plaintext and the entire body is the redirect location.

![](/example.png)

```
curl --request PUT \
  --url https://cdyn.dev/ \
  --header 'Authorization: Bearer K9h69DfTsTjVPGUrgg2o3fv5hjIglNoIfci1+kMFfyAg' \
  --header 'Content-Type: text/plain' \
  --data https://github.com/cendyne/sh
```

The response body will be the path, so a response with `/V` will mean that the host will redirect to the value `https://github.com/cendyne/sh` when visited at `/V`. Thus https://cdyn.dev/V will redirect to `https://github.com/cendyne/sh`

![](qr.png)

This project was originally made to enable QR small QR codes.
_Unfortunately, I cannot feasibly get a domain short enough for micro qr codes, and most device camera apps do not natively recognize micro qr codes._

```
echo -n "http://cdyn.dev/V" | qrencode -t png -l L -o qr.png
```

-----

This project has been replaced by https://github.com/cendyne/short-url, a cloudflare worker based solution
The API is a little different but it works on similar principals.
