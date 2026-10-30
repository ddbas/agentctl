# Docker image for isolated tmux integration tests.
#
# This image is used by TmuxEnv in tests/e2e/support/tmux-env.ts. It provides a
# minimal Alpine environment with tmux pre-installed so that tests never
# perform runtime package installation (which requires container internet
# access and is fragile on macOS Docker Desktop).
#
# The entrypoint is overridden at test startup; this CMD is never used
# directly by TmuxEnv but serves as documentation of the intended use.
FROM alpine:latest

RUN apk add --no-cache tmux bash

ENV TERM=xterm-256color
