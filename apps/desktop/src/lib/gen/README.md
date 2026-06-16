# Generated gRPC-Web client

These files are generated from `proto/gitdeck/v1/*.proto` (the single source of
truth for both the Rust server and this client). They are committed so the build
needs no codegen step.

**Do not edit by hand.** Regenerate after changing the proto:

```sh
cd apps/desktop
bunx buf generate
```

Toolchain (Connect-ES / protobuf-es v2 line):

- `@connectrpc/connect` + `@connectrpc/connect-web` (runtime transport/client)
- `@bufbuild/protobuf` (runtime)
- `@bufbuild/buf` + `@bufbuild/protoc-gen-es` (dev — codegen)

Config: `apps/desktop/buf.gen.yaml`. In v2, `protoc-gen-es` emits both the
message types and the service descriptor (`Gitdeck`), consumed by
`createClient(Gitdeck, transport)` — no separate connect plugin is needed.
