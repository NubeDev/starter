# hello-cli

Minimal CLI-flavour extension example. Contributes two subcommands:

- `hellocli-greet --name <NAME>` — prints `{"message":"hello, <NAME>"}`.
- `hellocli-tick --count <N>` — streams `N` JSON ticks to stdout, one
  per line. `Ctrl-C` cancels the stream cleanly (the kernel forwards
  `stream.cancel` to the handler within a few hundred ms).
