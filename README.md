Compiling for x86_64 linux on mac is difficult.

```
rustup target add x86_64-unknown-linux-gnu
brew tap SergioBenitez/osxct
brew install x86_64-unknown-linux-gnu
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo build --release --target=x86_64-unknown-linux-gnu
```

an `ALPHABET` environment variable makes things less guessable.

```
node
Welcome to Node.js v16.13.1.
Type ".help" for more information.
> str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
> str.split('').sort(function(){return 0.5-Math.random()}).join('');
'IGWpJdZgAzBQHaiw30OmV4ytsC7xnkeF1MUcfKDjq2lhTREvrLS5N68bP9XoYu'
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
