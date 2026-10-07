**example:**

```sh
carpenter link register
```

Result (one envelope on stdout):
```json
{"status":"ok","message":"link manifest emitted","data":{"name":"carpenter","version":"0.12.0","bin":"/…/carpenter","summary":"Agent-driven CLI that builds Python/Jupyter learning material.","howto_excerpt":"Run `carpenter howto` for the full, always-current command manual.","commands":["course","lesson","plan","quiz","howto"]}}
```

Future CLI registry manifest. Read-only emit.
